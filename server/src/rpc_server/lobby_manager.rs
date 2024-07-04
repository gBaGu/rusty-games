use std::marker::PhantomData;

use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tokio::task::JoinHandle;
use tonic::Streaming;

use super::error::RpcError;
use super::game_storage::GameStorage;
use super::lobby::{Connection, UpdateRequestReader};
use super::rpc::{GameImpl, RpcInnerResult, UserId};
use super::GameId;
use crate::game::{Game, GameState};
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

pub struct Worker<T>((JoinHandle<()>, PhantomData<T>));

impl<T> Worker<T>
where
    T: Game + Send + 'static,
{
    pub fn new(
        storage: GameStorage<T>,
        mut command_receiver: UnboundedReceiver<Option<WorkerCommand>>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            while let Some(command) = command_receiver.recv().await {
                let Some(command) = command else {
                    println!("worker: no more commands");
                    break;
                };
                match command {
                    WorkerCommand::UpdateGame { game, user, data } => {
                        println!(
                            "worker: UpdateGame game={}, user={}, data={:?}",
                            game, user, data
                        );
                        if let Err(err) = storage.update(game, user, &data) {
                            if let Err(err) = storage.notify_err(game, user, err) {
                                println!("worker: failed to notify on error: {}", err);
                            }
                        }
                    }
                    WorkerCommand::Disconnect { game, user } => {
                        println!("worker: Disconnect game={}, user={}", game, user);
                        match storage.disconnect(game, user) {
                            Ok(conn) => {
                                if let Some(mut conn) = conn {
                                    if let Err(err) = conn.wait().await {
                                        println!("worker: failed to join reader: {}", err);
                                    }
                                }
                            }
                            Err(err) => {
                                println!("worker: failed to disconnect user {}: {}", user, err);
                            }
                        }
                    }
                }
            }
        });
        Self((worker, Default::default()))
    }
}

pub struct LobbyManager<T> {
    storage: GameStorage<T>,
    command_sender: UnboundedSender<Option<WorkerCommand>>,
    worker: Worker<T>,
}

impl<T> Drop for LobbyManager<T> {
    fn drop(&mut self) {
        self.command_sender.send(None).unwrap(); // TODO: remove unwrap here
    }
}

impl<T> Default for LobbyManager<T>
where
    T: Game + Send + 'static,
{
    fn default() -> Self {
        let storage = GameStorage::default();
        let (s, r) = unbounded_channel();
        let worker = Worker::new(storage.clone(), r);
        Self {
            storage,
            command_sender: s,
            worker,
        }
    }
}

impl<T> LobbyManager<T> {
    pub fn command_sender(&self) -> UnboundedSender<Option<WorkerCommand>> {
        self.command_sender.clone()
    }

    pub fn create_connection(
        &self,
        game: GameId,
        user: UserId,
        stream: Streaming<proto::GameSessionRequest>,
    ) -> RpcInnerResult<UnboundedReceiver<RpcInnerResult<(UserId, Vec<u8>)>>> {
        let command_sender = self.command_sender();
        let (s, r) = unbounded_channel();
        let mut guard = self.storage.lock()?;
        let lobby = guard
            .get_mut(&game)
            .ok_or(RpcError::NoSuchGame { id: game })?;
        let request_reader = UpdateRequestReader::new(
            game,
            user,
            stream,
            command_sender,
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
            while let Some(data) = reply_receiver.recv().await {
                let (player_id, data) = data?;
                yield proto::GameSessionReply { player_id, turn_data: data };
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
