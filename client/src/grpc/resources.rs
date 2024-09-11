use std::time::Duration;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use game_server::core::ToProtobuf;
use game_server::proto;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::{Code, Request};
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::HealthCheckRequest;

use super::{CallTask, GameClient, HealthClient, CONNECT_INTERVAL_SEC, HEALTH_RETRY_INTERVAL_SEC};
use crate::grpc::error::GrpcError;

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
        self.connected = connected;
    }

    pub fn create_game(
        &self,
        game_type: proto::GameType,
        player_id: u64,
        opponent_id: u64,
    ) -> Result<CallTask<proto::CreateGameReply>, GrpcError> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .create_game(Request::new(proto::CreateGameRequest {
                    game_type: game_type.into(),
                    player_ids: vec![player_id, opponent_id],
                }))
                .await
        });
        Ok(CallTask::new(task))
    }

    pub fn make_turn(
        &self,
        game_type: proto::GameType,
        game_id: u64,
        player_id: u64,
        move_data: impl ToProtobuf,
    ) -> Result<CallTask<proto::MakeTurnReply>, GrpcError> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let move_data = move_data.to_protobuf()?;
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .make_turn(Request::new(proto::MakeTurnRequest {
                    game_type: game_type.into(),
                    game_id,
                    player_id,
                    turn_data: move_data,
                }))
                .await
        });
        Ok(CallTask::new(task))
    }

    pub fn get_game(
        &self,
        game_type: proto::GameType,
        game_id: u64,
    ) -> Result<CallTask<proto::GetGameReply>, GrpcError> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_game(Request::new(proto::GetGameRequest {
                    game_type: game_type.into(),
                    game_id,
                }))
                .await
        });
        Ok(CallTask::new(task))
    }

    pub fn get_player_games(
        &self,
        game_type: proto::GameType,
        player_id: u64,
    ) -> Result<CallTask<proto::GetPlayerGamesReply>, GrpcError> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_player_games(Request::new(proto::GetPlayerGamesRequest {
                    game_type: game_type.into(),
                    player_id,
                }))
                .await
        });
        Ok(CallTask::new(task))
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
