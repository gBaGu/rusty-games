mod app_state;
mod proto;
mod interface;
mod settings;

use bevy::prelude::{App, Camera2dBundle, Commands, DefaultPlugins, Resource, Startup};

use crate::app_state::AppState;
use crate::interface::plugin::InterfacePlugin;
use crate::settings::Settings;

pub struct Game {
    user_id: u64,
    opponent_id: u64,
}

#[derive(Default, Resource)]
pub struct CurrentGame(pub Option<Game>);

fn init_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    App::new()
        .init_state::<AppState>()
        .insert_resource(Settings::default())
        .insert_resource(CurrentGame::default())
        .add_plugins((DefaultPlugins, InterfacePlugin))
        .add_systems(Startup, init_camera)
        .run();
}
