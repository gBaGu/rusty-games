mod app_state;
mod game;
mod grpc;
mod interface;
mod settings;

use std::time::Duration;

use bevy::app::{App, Startup, Update};
use bevy::prelude::{Camera2dBundle, Commands, DefaultPlugins};
use bevy::time::{Timer, TimerMode};

use crate::app_state::AppState;
use crate::grpc::{GrpcClient, ReconnectTimer, RECONNECT_INTERVAL_SEC};
use crate::interface::plugin::InterfacePlugin;
use crate::settings::Settings;

fn init_camera(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
}

fn main() {
    let mut t = Timer::from_seconds(RECONNECT_INTERVAL_SEC, TimerMode::Repeating);
    t.set_elapsed(Duration::from_secs_f32(RECONNECT_INTERVAL_SEC));
    App::new()
        .init_state::<AppState>()
        .init_resource::<Settings>()
        .insert_resource(ReconnectTimer(t))
        .init_resource::<GrpcClient>()
        .add_plugins((DefaultPlugins, InterfacePlugin))
        .add_systems(Startup, init_camera)
        .add_systems(Update, (grpc::reconnect, grpc::handle_reconnect))
        .run();
}
