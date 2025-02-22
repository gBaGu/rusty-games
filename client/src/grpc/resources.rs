use std::time::Duration;

use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use game_server::core::{self, FromProtobuf as _, ToProtobuf as _};
use game_server::proto;
use tonic::codegen::tokio_stream::{self, StreamExt};
use tonic::metadata::errors::InvalidMetadataValue;
use tonic::metadata::{Ascii, MetadataValue};
use tonic::transport::Endpoint;
use tonic::{Code, Request};
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::HealthCheckRequest;

use super::components::{CallTask, GameSession, LogInTask};
use super::error::GrpcError;
use super::{
    AuthClient, GameClient, GameSessionUpdate, GrpcResult, HealthClient, CONNECT_INTERVAL_SEC,
    GAME_SESSION_CHECK_INTERVAL_SEC, HEALTH_RETRY_INTERVAL_SEC,
};

/// Endpoint of grpc server.
#[derive(Clone, Debug, Deref, Resource)]
pub struct ServerEndpoint(Endpoint);

impl ServerEndpoint {
    pub fn new(endpoint: Endpoint) -> Self {
        Self(endpoint)
    }
}

#[derive(Debug, Resource)]
pub struct GrpcClient {
    game: GameClient,
    auth: AuthClient,
    auth_metadata: Option<MetadataValue<Ascii>>,
    connected: bool,
}

impl GrpcClient {
    pub fn new(game: GameClient, auth: AuthClient) -> Self {
        Self {
            game,
            auth,
            auth_metadata: None,
            connected: true,
        }
    }

    pub fn connected(&self) -> bool {
        self.connected
    }

    pub fn set_connected(&mut self, connected: bool) {
        self.connected = connected;
    }

    pub fn store_token(&mut self, token: &str) -> Result<(), InvalidMetadataValue> {
        let new_meta = format!("Bearer {}", token).parse()?;
        let _ = self.auth_metadata.insert(new_meta);
        Ok(())
    }

    pub fn log_in(&self) -> GrpcResult<LogInTask> {
        if !self.connected {
            return Err(GrpcError::NotConnected);
        }
        let mut client = self.auth.clone();
        let (link_s, link_r) = async_channel::unbounded();
        let (token_s, token_r) = async_channel::unbounded();
        let task = IoTaskPool::get().spawn(async move {
            let mut reply = client.log_in(proto::LogInRequest {}).await?.into_inner();
            let link = reply.next().await.ok_or(GrpcError::ReplyStreamFinished)??;
            let Some(proto::log_in_reply::Reply::Link(link)) = link.reply else {
                return Err(GrpcError::InvalidReply(
                    "auth link is missing from reply".into(),
                ));
            };
            link_s.send(link).await?;
            let token = reply.next().await.ok_or(GrpcError::ReplyStreamFinished)??;
            let Some(proto::log_in_reply::Reply::Token(token)) = token.reply else {
                return Err(GrpcError::InvalidReply(
                    "token is missing from reply".into(),
                ));
            };
            token_s.send(token).await?;
            Ok(())
        });
        Ok(LogInTask::new(task, link_r, token_r))
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
        let auth_metadata = self.auth_metadata.clone();
        let task = IoTaskPool::get().spawn(async move {
            let mut request = Request::new(proto::CreateGameRequest::new(
                T::get_game_type().into(),
                vec![player_id, opponent_id],
            ));
            if let Some(meta) = auth_metadata {
                request.metadata_mut().insert("authorization", meta);
            }
            client.create_game(request).await
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
            let request = Request::new(proto::MakeTurnRequest::new(
                T::get_game_type().into(),
                game_id,
                player_id,
                move_data,
            ));
            client.make_turn(request).await
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
        let auth_metadata = self.auth_metadata.clone();
        let (action_s, action_r) = async_channel::unbounded::<Vec<u8>>();
        let (update_s, update_r) = async_channel::unbounded();
        let task = IoTaskPool::get().spawn(async move {
            let request_stream = tokio_stream::once(proto::GameSessionRequest::init(
                T::get_game_type().into(),
                game_id,
                player_id,
            ))
            .chain(action_r.map(|data| proto::GameSessionRequest::turn_data(data)));
            let mut request = Request::new(request_stream);
            if let Some(meta) = auth_metadata {
                request.metadata_mut().insert("authorization", meta);
            }
            let mut reply_stream = match client.game_session(request).await {
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
            let request = Request::new(proto::GetGameRequest::new(
                T::get_game_type().into(),
                game_id,
            ));
            client.get_game(request).await
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
            let request = Request::new(proto::GetPlayerGamesRequest::new(
                T::get_game_type().into(),
                player_id,
            ));
            client.get_player_games(request).await
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
