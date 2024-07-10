mod components;
mod resources;
mod systems;
mod task_entity;

use bevy::prelude::*;
use game_server::proto;
use tonic::transport;
use tonic_health::pb::health_client;

pub use components::{CallCreateGame, CallGetGame, CallGetPlayerGames, CallMakeTurn};
pub use resources::GrpcClient;
pub use task_entity::TaskEntity;

use components::{ConnectClient, ReceiveUpdate};
use resources::{ConnectTimer, ConnectionStatusWatcher};
use systems::*;

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
            .add_systems(
                Update,
                (
                    connect.run_if(
                        not(resource_exists::<GrpcClient>)
                            .and_then(not(any_with_component::<ConnectClient>)),
                    ),
                    handle_connect.run_if(any_with_component::<ConnectClient>),
                    receive_status.run_if(
                        resource_exists::<ConnectionStatusWatcher>
                            .and_then(not(any_with_component::<ReceiveUpdate>)),
                    ),
                    handle_receive_status.run_if(any_with_component::<ReceiveUpdate>),
                ),
            );
    }
}
