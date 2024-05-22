mod app_state;
mod game;
mod grpc;
mod interface;
mod settings;

use std::collections::HashMap;
use std::time::Duration;

use bevy::app::{App, Startup, Update};
use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{Camera2dBundle, Commands, DefaultPlugins, Image, Resource};
use bevy::time::{Timer, TimerMode};

use crate::app_state::AppState;
use crate::grpc::{GrpcClient, ReconnectTimer, RECONNECT_INTERVAL_SEC};
use crate::interface::plugin::InterfacePlugin;
use crate::settings::Settings;

pub struct Game {
    user_id: u64,
    opponent_id: u64,
    next: u64,
    images: HashMap<u64, Handle<Image>>,
}

impl Game {
    pub fn new(user_id: u64, opponent_id: u64, asset_server: &AssetServer) -> Self {
        let x_image = asset_server.load(interface::common::X_SPRITE_PATH);
        let o_image = asset_server.load(interface::common::O_SPRITE_PATH);
        Self {
            user_id,
            opponent_id,
            next: user_id,
            images: [(user_id, x_image), (opponent_id, o_image)]
                .into_iter()
                .collect(),
        }
    }
}

#[derive(Default, Resource)]
pub struct CurrentGame(pub Option<Game>);

fn init_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    let mut t = Timer::from_seconds(RECONNECT_INTERVAL_SEC, TimerMode::Repeating);
    t.set_elapsed(Duration::from_secs_f32(RECONNECT_INTERVAL_SEC));
    App::new()
        .init_state::<AppState>()
        .init_resource::<Settings>()
        .init_resource::<CurrentGame>()
        .insert_resource(ReconnectTimer(t))
        .init_resource::<GrpcClient>()
        .add_plugins((DefaultPlugins, InterfacePlugin))
        .add_systems(Startup, init_camera)
        .add_systems(Update, (grpc::reconnect, grpc::handle_reconnect))
        .run();
}
