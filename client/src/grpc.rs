use std::time::Duration;

use async_compat::CompatExt;
use bevy::prelude::{Component, Deref, DerefMut, Res, ResMut, Resource, TimerMode};
use bevy::tasks::{block_on, futures_lite::future, IoTaskPool, Task};
use bevy::time::{Time, Timer};
use game_server::rpc_server::rpc::RpcResult;
use tonic::transport;
use tonic::Request;

pub mod proto {
    tonic::include_proto!("game");
}

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "http://localhost:50051";
pub const RECONNECT_INTERVAL_SEC: f32 = 5.0;

pub type GameClient = proto::game_client::GameClient<transport::Channel>;

#[derive(Debug, Default, Deref, DerefMut, Resource)]
pub struct GrpcClient {
    #[deref]
    inner: Option<GameClient>,
    connect: Option<Task<Result<GameClient, transport::Error>>>,
}

impl GrpcClient {
    pub fn from_game_client(client: GameClient) -> Self {
        Self {
            inner: Some(client),
            connect: None,
        }
    }

    pub fn create_game(
        &self,
        player_id: u64,
        opponent_id: u64,
    ) -> Option<Task<RpcResult<proto::CreateGameReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
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

    pub fn load_player_games(
        &self,
        player_id: u64,
    ) -> Option<Task<RpcResult<proto::GetPlayerGamesReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
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

#[derive(Deref, DerefMut, Resource)]
pub struct ReconnectTimer(pub Timer);

impl Default for ReconnectTimer {
    fn default() -> Self {
        let mut t = Timer::from_seconds(RECONNECT_INTERVAL_SEC, TimerMode::Repeating);
        t.set_elapsed(Duration::from_secs_f32(RECONNECT_INTERVAL_SEC));
        Self(t)
    }
}

/// Task component for creating game over grpc
#[derive(Component, Deref)]
pub struct CallCreateGame(pub Task<RpcResult<proto::CreateGameReply>>);

/// Task component for loading player games over grpc
#[derive(Component, Deref)]
pub struct CallGetPlayerGames(pub Task<RpcResult<proto::GetPlayerGamesReply>>);

pub fn reconnect(
    mut client: ResMut<GrpcClient>,
    mut timer: ResMut<ReconnectTimer>,
    time: Res<Time>,
) {
    if client.is_some() {
        timer.reset();
        return;
    }

    if timer.tick(time.delta()).just_finished() && client.connect.is_none() {
        println!("reconnecting to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            GameClient::connect(DEFAULT_GRPC_SERVER_ADDRESS)
                .compat()
                .await
        });
        client.connect = Some(task);
    }
}

pub fn handle_reconnect(mut client: ResMut<GrpcClient>) {
    if let Some(task) = &mut client.connect {
        if let Some(res) = block_on(future::poll_once(task)) {
            match res {
                Ok(c) => *client = GrpcClient::from_game_client(c),
                Err(err) => {
                    println!("grpc client connect failed: {:?}", err);
                    client.connect.take();
                }
            }
        }
    }
}
