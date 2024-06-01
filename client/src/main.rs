mod app_state;
mod game;
mod grpc;
mod interface;
mod board;
mod commands;

use bevy::app::{App, Startup, Update};
use bevy::prelude::{Camera2dBundle, Commands, DefaultPlugins};

use crate::app_state::AppState;
use crate::board::BoardPlugin;
use crate::game::GamePlugin;
use crate::grpc::{GrpcClient, ReconnectTimer};
use crate::interface::InterfacePlugin;

fn init_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    App::new()
        .init_state::<AppState>()
        .init_resource::<ReconnectTimer>()
        .init_resource::<GrpcClient>()
        .add_plugins((DefaultPlugins, BoardPlugin, GamePlugin, InterfacePlugin))
        .add_systems(Startup, init_camera)
        .add_systems(Update, (grpc::reconnect, grpc::handle_reconnect))
        .run();
}
