mod app_state;
mod commands;
mod events;
mod game;
mod grpc;
mod interface;
mod resources;
mod systems;

use bevy::prelude::*;
use clap::Parser;

use app_state::AppState;
use game::GamePlugin;
use grpc::GrpcPlugin;
use interface::InterfacePlugin;

pub use events::UserIdChanged;
pub use resources::Settings;

#[derive(Parser)]
#[command(version)]
#[command(about = "Set of board games", long_about = None)]
struct Cli {
    #[arg(long)]
    user_id: Option<u64>,
}

#[derive(Component)]
pub struct Background;

fn main() {
    let cli = Cli::parse();
    let mut settings = Settings::default();
    if let Some(user_id) = cli.user_id {
        settings.set_user_id(user_id);
        println!("user_id is initialized to {}", user_id);
    }

    App::new()
        .add_plugins((DefaultPlugins, GamePlugin, GrpcPlugin, InterfacePlugin))
        .init_state::<AppState>()
        .insert_resource(settings)
        .add_event::<UserIdChanged>()
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .run();
}
