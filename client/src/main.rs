mod app_state;
mod grpc;
mod interface;
mod settings;

use std::collections::HashMap;

use bevy::asset::{AssetServer, Handle};
use bevy::prelude::{App, Camera2dBundle, Commands, DefaultPlugins, Image, Resource, Startup};
use bevy::tasks::block_on;

use crate::app_state::AppState;
use crate::grpc::{GameClient, GrpcClient, DEFAULT_GRPC_SERVER_ADDRESS};
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let grpc_client = match block_on(GameClient::connect(DEFAULT_GRPC_SERVER_ADDRESS)) {
        Ok(client) => GrpcClient::from_game_client(client),
        Err(err) => {
            println!("grpc client connect failed: {:?}", err);
            GrpcClient::new()
        }
    };
    App::new()
        .init_state::<AppState>()
        .init_resource::<Settings>()
        .init_resource::<CurrentGame>()
        .insert_resource(grpc_client)
        .add_plugins((DefaultPlugins, InterfacePlugin))
        .add_systems(Startup, init_camera)
        .run();
    Ok(())
}
