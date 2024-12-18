use std::future::{Future, IntoFuture};

use tokio::select;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tonic::Streaming;

use super::error::RpcError;
use super::game_storage::GameStorage;
use super::lobby::{Connection, MoveEvent, UpdateRequestReader};
use super::rpc::{GameImpl, RpcInnerResult, UserId};
use super::GameId;
use crate::core::{Game, GameState};
use crate::proto;

#[derive(Debug)]
pub enum WorkerCommand {
    UpdateGame {
        game: GameId,
        user: UserId,
        data: Vec<u8>,
    },
    Disconnect {
        game: GameId,
        user: UserId,
    },
}

pub struct Worker(JoinHandle<()>);

impl IntoFuture for Worker {
    type Output = <JoinHandle<()> as Future>::Output;
    type IntoFuture = JoinHandle<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.0.into_future()
    }
}

impl Worker {
    pub fn new<T: Game + Send + 'static>(
        storage: GameStorage<T>,
        mut command_receiver: UnboundedReceiver<WorkerCommand>,
        ct: CancellationToken,
    ) -> Self {
        let worker = tokio::spawn(async move {
            loop {
                select! {
                    biased;
                    _ = ct.cancelled() => {
                        command_receiver.close();
                        println!("worker: cancelled");
                        break;
                    },
                    v = command_receiver.recv() => {
                        let Some(command) = v else {
                            break;
                        };
                        match command {
                            WorkerCommand::UpdateGame { game, user, data } => {
                                println!(
                                    "worker: UpdateGame game={}, user={}, data={:?}",
                                    game, user, data
                                );
                                if let Err(err) = storage.update(game, user, &data) {
                                    println!("worker: UpdateGame failed: {}", err);
                                    if let Err(err) = storage.notify_err(game, user, err) {
                                        println!("worker: failed to notify on error: {}", err);
                                    }
                                    if let Err(err) = storage.disconnect(game, user).await {
                                        println!("worker: failed to disconnect on error: {}", err);
                                    }
                                }
                            }
                            WorkerCommand::Disconnect { game, user } => {
                                println!("worker: Disconnect game={}, user={}", game, user);
                                if let Err(err) = storage.disconnect(game, user).await {
                                    println!("worker: Disconnect failed: {}", err);
                                }
                            }
                        }
                    }
                }
            }
            println!("worker: finished");
        });
        Self(worker)
    }
}

pub struct LobbyManager<T> {
    storage: GameStorage<T>,
    command_sender: Option<UnboundedSender<WorkerCommand>>,
}

impl<T> Default for LobbyManager<T> {
    fn default() -> Self {
        Self {
            storage: Default::default(),
            command_sender: None,
        }
    }
}

impl<T> LobbyManager<T>
where
    T: Game + Send + 'static,
{
    pub fn start_worker(&mut self, ct: CancellationToken) -> Worker {
        let (s, r) = unbounded_channel();
        let _ = self.command_sender.insert(s);
        Worker::new(self.storage.clone(), r, ct)
    }
}

impl<T> LobbyManager<T> {
    pub fn command_sender(&self) -> &Option<UnboundedSender<WorkerCommand>> {
        &self.command_sender
    }

    pub fn create_connection(
        &self,
        game: GameId,
        user: UserId,
        stream: Streaming<proto::GameSessionRequest>,
    ) -> RpcInnerResult<UnboundedReceiver<RpcInnerResult<MoveEvent>>> {
        let Some(command_sender) = self.command_sender() else {
            return Err(RpcError::WorkerDown);
        };
        let (s, r) = unbounded_channel();
        let mut guard = self.storage.lock()?;
        let lobby = guard
            .get_mut(&game)
            .ok_or(RpcError::NoSuchGame { id: game })?;
        let request_reader = UpdateRequestReader::new(
            game,
            user,
            stream,
            command_sender.clone(),
            s.clone(),
            lobby.reader_cancellation_token(),
        );
        lobby.add_connection(Connection::new(user, request_reader, s));
        Ok(r)
    }

    pub fn start_game_session(
        &self,
        game: GameId,
        user: UserId,
        stream: Streaming<proto::GameSessionRequest>,
    ) -> RpcInnerResult<<GameImpl as proto::game_server::Game>::GameSessionStream> {
        // TODO: consider replacing proto type
        let mut reply_receiver = self.create_connection(game, user, stream)?;
        let reply_stream = async_stream::try_stream! {
            while let Some(event) = reply_receiver.recv().await {
                let MoveEvent { player, data } = event?;
                yield proto::GameSessionReply { player_position: player, turn_data: data };
            }
        };
        Ok(Box::pin(reply_stream))
    }
}

impl<T: Game> LobbyManager<T> {
    pub fn create(&self, id: GameId, players: &[UserId]) -> RpcInnerResult<proto::GameInfo> {
        self.storage.create(id, players)
    }

    pub fn update(&self, game: GameId, user: UserId, data: &[u8]) -> RpcInnerResult<GameState> {
        self.storage.update(game, user, data)
    }

    pub fn delete(&self, id: GameId) -> RpcInnerResult<()> {
        self.storage.delete(id)
    }

    pub fn get_game(&self, id: GameId) -> RpcInnerResult<proto::GameInfo> {
        self.storage.get(id)
    }

    pub fn get_player_games(&self, player: UserId) -> RpcInnerResult<Vec<proto::GameInfo>> {
        self.storage.get_player_games(player)
    }
}
