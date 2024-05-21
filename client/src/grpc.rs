use bevy::prelude::{Component, Deref, DerefMut, Entity, Resource};
use bevy::tasks::Task;
use std::ops::{Deref, DerefMut};
use tonic::transport;

use game_server::rpc_server::rpc::RpcResult;
pub use proto::game_client::GameClient;

pub mod proto {
    tonic::include_proto!("game");
}

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "localhost:50051";

#[derive(Debug, Default, Deref, DerefMut, Resource)]
pub struct GrpcClient {
    inner: Option<GameClient<transport::Channel>>,
}

impl GrpcClient {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_game_client(client: GameClient<transport::Channel>) -> Self {
        Self {
            inner: Some(client),
        }
    }
}

/// Task component for loading player games over grpc
#[derive(Component, Deref)]
pub struct CallGetPlayerGames(Task<RpcResult<proto::GetPlayerGamesReply>>);
