use std::time::Duration;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use game_server::core::{self, FromProtobuf as _, ToProtobuf as _};
use game_server::proto;
use tonic::codegen::tokio_stream::{self, StreamExt};
use tonic::{Code, Request};
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::HealthCheckRequest;

use super::error::GrpcError;
use super::{
    CallTask, GameClient, GameSession, GameSessionUpdate, GrpcResult, HealthClient,
    CONNECT_INTERVAL_SEC, GAME_SESSION_CHECK_INTERVAL_SEC, HEALTH_RETRY_INTERVAL_SEC,
};

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

    pub fn create_game<T: proto::GetGameType>(
        &self,
        player_id: u64,
        opponent_id: u64,
    ) -> GrpcResult<CallTask<proto::CreateGameReply>> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .create_game(Request::new(proto::CreateGameRequest {
                    game_type: T::get_game_type().into(),
                    player_ids: vec![player_id, opponent_id],
                }))
                .await
        });
        Ok(task.into())
    }

    pub fn _make_turn<T>(
        &self,
        game_id: u64,
        player_id: u64,
        move_data: T::TurnData,
    ) -> GrpcResult<CallTask<proto::MakeTurnReply>>
    where
        T: core::Game + proto::GetGameType,
    {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let move_data = move_data.to_protobuf()?;
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .make_turn(Request::new(proto::MakeTurnRequest {
                    game_type: T::get_game_type().into(),
                    game_id,
                    player_id,
                    turn_data: move_data,
                }))
                .await
        });
        Ok(task.into())
    }

    pub fn game_session<T>(
        &self,
        game_id: u64,
        player_id: u64,
    ) -> GrpcResult<GameSession<T, T::TurnData>>
    where
        T: core::Game + proto::GetGameType,
        T::TurnData: Send + 'static,
    {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let (action_s, action_r) = async_channel::unbounded::<Vec<u8>>();
        let (update_s, update_r) = async_channel::unbounded();
        let task = IoTaskPool::get().spawn(async move {
            let request_stream = tokio_stream::once(proto::GameSessionRequest {
                request: Some(proto::game_session_request::Request::Init(
                    proto::GameSession {
                        game_type: T::get_game_type().into(),
                        game_id,
                        player_id,
                    },
                )),
            })
            .chain(action_r.map(|data| proto::GameSessionRequest {
                request: Some(proto::game_session_request::Request::TurnData(data)),
            }));
            let mut reply_stream = match client.game_session(request_stream).await {
                Ok(resp) => resp.into_inner(),
                Err(err) => {
                    error!("failed to execute GameSession request: {}", err);
                    return;
                }
            };
            while let Some(res) = reply_stream.next().await {
                let update_result = match res {
                    Ok(reply) => match T::TurnData::from_protobuf(&reply.turn_data) {
                        Ok(action) => Ok(GameSessionUpdate::new(reply.player_position, action)),
                        Err(err) => Err(err.into()),
                    },
                    Err(err) => Err(GrpcError::GameSessionUpdateFailed(err.to_string())),
                };
                if let Err(_err) = update_s.send(update_result).await {
                    debug!("update channel is closed, skipping next updates...");
                    break;
                }
            }
            debug!("game session task is finished");
        });
        Ok(GameSession::new(task, action_s, update_r))
    }

    pub fn get_game<T: proto::GetGameType>(
        &self,
        game_id: u64,
    ) -> GrpcResult<CallTask<proto::GetGameReply>> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_game(Request::new(proto::GetGameRequest {
                    game_type: T::get_game_type().into(),
                    game_id,
                }))
                .await
        });
        Ok(task.into())
    }

    pub fn get_player_games<T: proto::GetGameType>(
        &self,
        player_id: u64,
    ) -> GrpcResult<CallTask<proto::GetPlayerGamesReply>> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.game.clone();
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_player_games(Request::new(proto::GetPlayerGamesRequest {
                    game_type: T::get_game_type().into(),
                    player_id,
                }))
                .await
        });
        Ok(task.into())
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
                                    error!("health client watch reply error: {}", err);
                                    false
                                }
                            };
                            if let Err(err) = s.send(status).await {
                                error!("unable to send connection status update: {}", err);
                            }
                        }
                    }
                    Err(status) if status.code() == Code::Unimplemented => break,
                    Err(status) => {
                        error!("health client watch request failed: {}", status);
                        if let Err(err) = s.send(false).await {
                            error!("unable to send connection status update: {}", err);
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

#[derive(Deref, DerefMut, Resource)]
pub struct SessionCheckTimer(pub Timer);

impl Default for SessionCheckTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_SESSION_CHECK_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}
