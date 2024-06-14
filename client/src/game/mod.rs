mod components;
mod error;
mod events;
mod game_info;
mod resources;
mod systems;

use bevy::prelude::*;

pub use components::Position;
pub use events::{
    CellUpdated, GameOver, LocalGameTurn, NetworkGameTurn, StateUpdated, SuccessfulTurn,
};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::{Authority, CurrentGame, GameType, LocalGame};

use crate::game::systems::{make_turn_local, make_turn_network};
use crate::grpc::CallGetGame;
use resources::RefreshGameTimer;
use systems::{
    game_initialized, handle_make_turn, handle_state_updated, refresh_game, update_game,
};

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub const BOARD_SIZE: usize = 3;
const BOARD_PROTO_SIZE: usize = BOARD_SIZE * BOARD_SIZE;
const PLAYERS_SIZE: usize = 2;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateUpdated>()
            .add_event::<CellUpdated>()
            .add_event::<NetworkGameTurn>()
            .add_event::<LocalGameTurn>()
            .add_event::<SuccessfulTurn>()
            .add_event::<GameOver>()
            .init_resource::<RefreshGameTimer>()
            .add_systems(
                Update,
                (
                    game_initialized.run_if(resource_added::<CurrentGame>),
                    refresh_game.run_if(
                        not(any_with_component::<CallGetGame>)
                            .and_then(resource_exists::<CurrentGame>)
                            .and_then(not(resource_exists::<LocalGame>)),
                    ),
                    (make_turn_network, make_turn_local).run_if(resource_exists::<CurrentGame>),
                    update_game,
                    handle_make_turn,
                    handle_state_updated,
                ),
            );
    }
}
