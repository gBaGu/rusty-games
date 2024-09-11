mod components;
mod events;
mod resources;
mod systems;
mod task_entity;
mod error;

use bevy::prelude::*;
use game_server::proto;
use tonic::transport;
use tonic_health::pb::health_client;

use components::{CallTask, ConnectClientTask, ReceiveUpdateTask};
use resources::{ConnectTimer, ConnectionStatusWatcher};
use systems::*;

pub use events::{Connected, Disconnected, RpcResultReady};
pub use resources::GrpcClient;

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "http://localhost:50051";
pub const CONNECT_INTERVAL_SEC: f32 = 5.0;
pub const HEALTH_RETRY_INTERVAL_SEC: f32 = 5.0;

pub type GameClient = proto::game_client::GameClient<transport::Channel>;
pub type HealthClient = health_client::HealthClient<transport::Channel>;

pub fn client_exists_and_connected(client: Option<Res<GrpcClient>>) -> bool {
    matches!(client, Some(c) if c.connected())
}

/// System set for network communication systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkSystems;

pub struct GrpcPlugin;

impl Plugin for GrpcPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ConnectTimer>()
            .configure_sets(Update, NetworkSystems.run_if(client_exists_and_connected))
            .add_event::<Connected>()
            .add_event::<Disconnected>()
            .add_event::<RpcResultReady<proto::CreateGameReply>>()
            .add_event::<RpcResultReady<proto::MakeTurnReply>>()
            .add_event::<RpcResultReady<proto::GetGameReply>>()
            .add_event::<RpcResultReady<proto::GetPlayerGamesReply>>()
            .add_systems(
                Update,
                (
                    connect.run_if(
                        not(resource_exists::<GrpcClient>)
                            .and_then(not(any_with_component::<ConnectClientTask>)),
                    ),
                    handle_connect.run_if(any_with_component::<ConnectClientTask>),
                    receive_status.run_if(
                        resource_exists::<ConnectionStatusWatcher>
                            .and_then(not(any_with_component::<ReceiveUpdateTask>)),
                    ),
                    handle_receive_status.run_if(any_with_component::<ReceiveUpdateTask>),
                    handle_response::<proto::CreateGameReply>
                        .run_if(any_with_component::<CallTask<proto::CreateGameReply>>),
                    handle_response::<proto::MakeTurnReply>
                        .run_if(any_with_component::<CallTask<proto::MakeTurnReply>>),
                    handle_response::<proto::GetGameReply>
                        .run_if(any_with_component::<CallTask<proto::GetGameReply>>),
                    handle_response::<proto::GetPlayerGamesReply>
                        .run_if(any_with_component::<CallTask<proto::GetPlayerGamesReply>>),
                ),
            );
    }
}
