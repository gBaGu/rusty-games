use async_compat::CompatExt;
use bevy::prelude::{Component, Deref, DerefMut, Res, ResMut, Resource};
use bevy::tasks::{block_on, futures_lite::future, IoTaskPool, Task};
use bevy::time::{Time, Timer};
use tonic::transport;
use tonic::Request;

use game_server::rpc_server::rpc::RpcResult;

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

#[derive(Default, Deref, DerefMut, Resource)]
pub struct ReconnectTimer(pub Timer);

/// Task component for loading player games over grpc
#[derive(Component, Deref)]
pub struct CallGetPlayerGames(pub Task<RpcResult<proto::GetPlayerGamesReply>>);

pub fn reconnect(mut client: ResMut<GrpcClient>, mut timer: ResMut<ReconnectTimer>, time: Res<Time>) {
    if client.is_some() {
        timer.reset();
        return;
    }

    if timer.tick(time.delta()).just_finished() && client.connect.is_none() {
        println!("reconnecting to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            GameClient::connect(DEFAULT_GRPC_SERVER_ADDRESS).compat().await
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
