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
use board::BoardPlugin;
use bot::BotPlugin;
use game::GamePlugin;
use grpc::GrpcPlugin;
use interface::InterfacePlugin;

#[derive(Component)]
pub struct Background;

fn main() {
    App::new()
        .init_state::<AppState>()
        .init_resource::<Settings>()
        .add_plugins((
            DefaultPlugins,
            BoardPlugin,
            BotPlugin,
            GamePlugin,
            GrpcPlugin,
            InterfacePlugin,
        ))
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .run();
}
