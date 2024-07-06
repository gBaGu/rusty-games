use std::ops::{Deref, DerefMut};

use smallvec::SmallVec;
use tokio::select;
use tokio::sync::mpsc::{error::SendError, UnboundedSender};
use tokio::task::{JoinError, JoinHandle};
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;
use tonic::Streaming;

use super::error::RpcError;
use super::lobby_manager::WorkerCommand;
use super::rpc::{RpcInnerResult, UserId};
use super::GameId;
use crate::game::encoding::FromProtobuf;
use crate::game::{Game, GameState, PlayerId};
use crate::proto::{game_session_request, GameSessionRequest};

type ChannelSendResult<T> = Result<(), SendError<T>>;

#[derive(Debug)]
pub struct MoveEvent {
    pub player: PlayerId,
    pub data: Vec<u8>,
}

impl MoveEvent {
    pub fn new(player: PlayerId, data: Vec<u8>) -> Self {
        Self { player, data }
    }
}

/// Thread that reads update data from input stream and sends it to worker
#[derive(Debug)]
pub struct UpdateRequestReader(JoinHandle<()>);

impl Deref for UpdateRequestReader {
    type Target = JoinHandle<()>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UpdateRequestReader {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl UpdateRequestReader {
    pub fn new(
        game: GameId,
        user: UserId,
        mut stream: Streaming<GameSessionRequest>,
        command_sender: UnboundedSender<WorkerCommand>,
        reply_sender: UnboundedSender<RpcInnerResult<MoveEvent>>,
        cancellation_token: CancellationToken,
    ) -> Self {
        let reader_thread = tokio::spawn(async move {
            loop {
                select! {
                    biased;
                    _ = cancellation_token.cancelled() => break,
                    v = stream.next() => {
                        let Some(res) = v else {
                            break;
                        };
                        match res {
                            Ok(request) => {
                                let Some(request) = request.request else {
                                    if let Err(err) = reply_sender.send(Err(RpcError::EmptyRequest)) {
                                        println!("failed to send error to client: {}", err);
                                    }
                                    continue;
                                };
                                match request {
                                    game_session_request::Request::TurnData(data) => {
                                        let update = WorkerCommand::UpdateGame { game, user, data };
                                        if let Err(err) = command_sender.send(update) {
                                            if let Err(err) = reply_sender.send(Err(err.into())) {
                                                println!("failed to send error to client: {}", err);
                                            }
                                        }
                                    }
                                    _ => {
                                        if let Err(err) = reply_sender.send(Err(
                                            RpcError::unexpected_request("TurnData", request.name()),
                                        )) {
                                            println!("failed to send error to client: {}", err);
                                            continue;
                                        }
                                    }
                                }
                            }
                            Err(err) => {
                                if let Err(err) = reply_sender.send(Err(err.into())) {
                                    println!("failed to send error to client: {}", err);
                                }
                            }
                        }
                    }
                }
            }
            if let Err(err) = command_sender.send(WorkerCommand::Disconnect { game, user }) {
                if let Err(err) = reply_sender.send(Err(err.into())) {
                    println!("failed to send error to client: {}", err);
                }
            }
        });
        Self(reader_thread)
    }
}

#[derive(Debug)]
pub struct Connection {
    id: UserId,
    request_reader: UpdateRequestReader,
    reply_sender: UnboundedSender<RpcInnerResult<MoveEvent>>,
}

impl Connection {
    pub fn new(
        user: UserId,
        request_reader: UpdateRequestReader,
        reply_sender: UnboundedSender<RpcInnerResult<MoveEvent>>,
    ) -> Self {
        Self {
            id: user,
            request_reader,
            reply_sender,
        }
    }

    pub fn notify(
        &self,
        player: PlayerId,
        data: Vec<u8>,
    ) -> ChannelSendResult<RpcInnerResult<MoveEvent>> {
        self.reply_sender.send(Ok(MoveEvent::new(player, data)))
    }

    pub fn notify_err(&self, err: RpcError) -> ChannelSendResult<RpcInnerResult<MoveEvent>> {
        self.reply_sender.send(Err(err))
    }

    pub async fn wait(&mut self) -> Result<(), JoinError> {
        self.request_reader.deref_mut().await
    }
}

#[derive(Debug, Default)]
pub struct Lobby<T> {
    players: SmallVec<[UserId; 8]>,
    game: T,
    connections: Vec<Connection>,
    reader_cancellation_token: CancellationToken,
}

impl<T> Lobby<T> {
    pub fn game(&self) -> &T {
        &self.game
    }

    pub fn players(&self) -> &[UserId] {
        self.players.as_slice()
    }

    pub fn reader_cancellation_token(&self) -> CancellationToken {
        self.reader_cancellation_token.clone()
    }

    pub fn get_player_position(&self, user: UserId) -> Option<usize> {
        self.players().iter().position(|&id| id == user)
    }

    pub fn add_connection(&mut self, conn: Connection) {
        self.connections.push(conn);
    }

    pub fn disconnect(&mut self, user: UserId) -> Option<Connection> {
        let Some((i, _)) = self
            .connections
            .iter()
            .enumerate()
            .find(|(_, conn)| conn.id == user)
        else {
            return None;
        };
        Some(self.connections.remove(i))
    }

    pub fn notify_err(&self, user: UserId, err: RpcError) -> RpcInnerResult<()> {
        if let Some(conn) = self.connections.iter().find(|conn| conn.id == user) {
            conn.notify_err(err)?;
        }
        Ok(())
    }
}

impl<T: Game> Lobby<T> {
    pub fn new(players: &[UserId]) -> Self {
        Self {
            players: SmallVec::from_slice(players),
            game: T::new(),
            connections: Default::default(),
            reader_cancellation_token: Default::default(),
        }
    }

    pub fn update(&mut self, player: UserId, data: &[u8]) -> RpcInnerResult<GameState> {
        let decoded_data = T::TurnData::from_protobuf(data)?;
        let player_position = self
            .get_player_position(player)
            .ok_or(RpcError::ForeignGame)?
            .try_into()
            .map_err(|err| {
                RpcError::internal(format!(
                    "failed to convert usize to player position: {}",
                    err
                ))
            })?;
        let state = self.game.update(player_position, decoded_data)?;
        for conn in self.connections.iter() {
            if let Err(err) = conn.notify(player_position, data.to_vec()) {
                println!("failed to notify subscriber: {}", err);
            }
        }
        if matches!(state, GameState::Finished(_)) {
            self.reader_cancellation_token.cancel();
        }
        Ok(state)
    }
}
