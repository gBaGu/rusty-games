use std::time::Duration;

use bevy::prelude::*;
use bevy::tasks::{IoTaskPool, Task};
use game_server::proto;
use game_server::rpc_server::rpc::RpcResult;
use prost::Message;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::{Code, Request};
use tonic_health::pb::{health_check_response::ServingStatus, HealthCheckRequest};

use super::{GameClient, HealthClient, CONNECT_INTERVAL_SEC, HEALTH_RETRY_INTERVAL_SEC};

#[derive(Debug, Resource)]
pub struct GrpcClient {
    game: GameClient,
    connected: bool,
}

impl GrpcClient {
    pub fn new(game: GameClient) -> Self {
        Self {
            game,
            connected: true,
        }
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn set_connected(&mut self, connected: bool) {
        if self.connected != connected {
            println!("connection status changed: {}", connected);
        }
        self.connected = connected;
    }

    pub fn create_game(
        &self,
        player_id: u64,
        opponent_id: u64,
    ) -> Option<Task<RpcResult<proto::CreateGameReply>>> {
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .create_game(Request::new(proto::CreateGameRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    player_ids: vec![player_id, opponent_id],
                }))
                .await
        });
        Some(task)
    }

    pub fn make_turn(
        &self,
        game_id: u64,
        player_id: u64,
        row: u32,
        col: u32,
    ) -> Option<Task<RpcResult<proto::MakeTurnReply>>> {
        let mut client = self.game.clone();
        let position = proto::Position { row, col };
        let task = IoTaskPool::get().spawn(async move {
            client
                .make_turn(Request::new(proto::MakeTurnRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    game_id,
                    player_id,
                    turn_data: position.encode_to_vec(),
                }))
                .await
        });
        Some(task)
    }

    pub fn load_game(&self, game_id: u64) -> Option<Task<RpcResult<proto::GetGameReply>>> {
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_game(Request::new(proto::GetGameRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    game_id,
                }))
                .await
        });
        Some(task)
    }

    pub fn load_player_games(
        &self,
        player_id: u64,
    ) -> Option<Task<RpcResult<proto::GetPlayerGamesReply>>> {
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_player_games(Request::new(proto::GetPlayerGamesRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    player_id,
                }))
                .await
        });
        Some(task)
    }
}

#[derive(Resource)]
pub struct ConnectionStatusWatcher {
    update_receiver: async_channel::Receiver<bool>,
}

impl ConnectionStatusWatcher {
    pub fn start(mut health_client: HealthClient) -> Self {
        let (s, r) = async_channel::unbounded();
        let watch_task = IoTaskPool::get().spawn(async move {
            loop {
                let res = health_client
                    .watch(Request::new(HealthCheckRequest {
                        service: "".to_string(),
                    }))
                    .await;
                match res {
                    Ok(response) => {
                        let mut stream = response.into_inner();
                        while let Some(res) = stream.next().await {
                            let status = match res {
                                Ok(response) => {
                                    response.status
                                        == <ServingStatus as Into<i32>>::into(
                                            ServingStatus::Serving,
                                        )
                                }
                                Err(err) => {
                                    println!("got error from health client watch stream: {}", err);
                                    false
                                }
                            };
                            if let Err(err) = s.send(status).await {
                                println!("unable to send connection status update: {}", err);
                            }
                        }
                    }
                    Err(status) if status.code() == Code::Unimplemented => break,
                    Err(status) => {
                        println!("got error from health service: {}", status);
                        if let Err(err) = s.send(false).await {
                            println!("unable to send connection status update: {}", err);
                        }
                    }
                }
                async_io::Timer::after(Duration::from_secs_f32(HEALTH_RETRY_INTERVAL_SEC)).await;
            }
        });
        watch_task.detach();
        Self { update_receiver: r }
    }

    pub fn update_receiver(&self) -> async_channel::Receiver<bool> {
        self.update_receiver.clone()
    }
}

#[derive(Deref, DerefMut, Resource)]
pub struct ConnectTimer(pub Timer);

impl Default for ConnectTimer {
    fn default() -> Self {
        let mut t = Timer::from_seconds(CONNECT_INTERVAL_SEC, TimerMode::Repeating);
        t.set_elapsed(Duration::from_secs_f32(CONNECT_INTERVAL_SEC));
        Self(t)
    }
}
