mod app_state;
mod commands;
mod game;
mod grpc;
mod interface;
mod resources;
mod systems;

use bevy::prelude::*;

pub use resources::Settings;

use app_state::AppState;
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
            GamePlugin,
            GrpcPlugin,
            InterfacePlugin,
        ))
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .run();
}
