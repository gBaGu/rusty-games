mod app_state;
mod board;
mod bot;
mod commands;
mod game;
mod grpc;
mod interface;
mod resources;
mod systems;

use bevy::prelude::*;

pub use resources::Settings;

use crate::app_state::AppState;
use crate::board::BoardPlugin;
use crate::bot::BotPlugin;
use crate::game::GamePlugin;
use crate::grpc::{
    ConnectClient, ConnectTimer, ConnectionStatusWatcher, GrpcClient, NetworkSystems, ReceiveUpdate,
};
use crate::interface::InterfacePlugin;

#[derive(Component)]
pub struct Background;

fn main() {
    App::new()
        .init_state::<AppState>()
        .init_resource::<ConnectTimer>()
        .init_resource::<Settings>()
        .configure_sets(
            Update,
            NetworkSystems.run_if(grpc::client_exists_and_connected),
        )
        .add_plugins((
            DefaultPlugins,
            BoardPlugin,
            BotPlugin,
            GamePlugin,
            InterfacePlugin,
        ))
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .add_systems(
            Update,
            (
                grpc::connect.run_if(
                    not(resource_exists::<GrpcClient>)
                        .and_then(not(any_with_component::<ConnectClient>)),
                ),
                grpc::handle_connect.run_if(any_with_component::<ConnectClient>),
                grpc::receive_status.run_if(
                    resource_exists::<ConnectionStatusWatcher>
                        .and_then(not(any_with_component::<ReceiveUpdate>)),
                ),
                grpc::handle_receive_status.run_if(any_with_component::<ReceiveUpdate>),
            ),
        )
        .run();
}
