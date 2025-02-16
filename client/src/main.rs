mod app_state;
mod commands;
mod common;
mod events;
mod game;
mod grpc;
mod interface;
mod resources;
mod systems;
mod util;

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
    /// ID of a logged-in user
    #[arg(long)]
    user_id: Option<u64>,
    /// Path to a file containing CA certificate
    #[arg(long)]
    ca_cert_path: Option<std::path::PathBuf>,
}

#[derive(Component)]
pub struct Background;

fn main() {
    let cli = Cli::parse();
    let mut settings = Settings::default();
    if let Some(user_id) = cli.user_id {
        settings.set_user_id(user_id);
    }

    let mut app = App::new();
    app.add_plugins(DefaultPlugins);
    if let Some(ca_cert_path) = cli.ca_cert_path {
        if let Ok(cert) = std::fs::read_to_string(ca_cert_path) {
            app.add_plugins(GrpcPlugin::new(&cert));
        } else {
            warn!("unable to read CA certificate - grpc plugin disabled");
        }
    }
    app.add_plugins((GamePlugin, InterfacePlugin))
        .init_state::<AppState>()
        .insert_resource(settings)
        .add_event::<UserIdChanged>()
        .add_systems(Startup, systems::init_app)
        .add_systems(Update, systems::on_resize)
        .run();
}
