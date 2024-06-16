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
use crate::grpc::{GrpcClient, ReconnectTimer};
use crate::interface::InterfacePlugin;

#[derive(Component)]
pub struct Background;

fn main() {
    App::new()
        .init_state::<AppState>()
        .init_resource::<ReconnectTimer>()
        .init_resource::<Settings>()
        .init_resource::<GrpcClient>()
        .add_plugins((
            DefaultPlugins,
            BoardPlugin,
            BotPlugin,
            GamePlugin,
            InterfacePlugin,
        ))
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .add_systems(Update, (grpc::reconnect, grpc::handle_reconnect))
        .run();
}
