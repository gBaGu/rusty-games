mod components;
mod events;
mod game_info;
mod resources;
mod systems;

use bevy::prelude::*;

pub use components::Position;
pub use events::{CellUpdated, GameOver, StateUpdated, SuccessfulTurn};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::CurrentGame;

use crate::grpc::CallGetGame;
use resources::RefreshGameTimer;
use systems::{game_initialized, handle_state_updated, handle_turn, refresh_game, update_game};

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateUpdated>()
            .add_event::<CellUpdated>()
            .add_event::<SuccessfulTurn>()
            .add_event::<GameOver>()
            .init_resource::<RefreshGameTimer>()
            .add_systems(
                Update,
                (
                    game_initialized.run_if(resource_added::<CurrentGame>),
                    refresh_game.run_if(
                        not(any_with_component::<CallGetGame>)
                            .and_then(resource_exists::<CurrentGame>),
                    ),
                    handle_turn,
                    update_game,
                    handle_state_updated,
                ),
            );
    }
}
