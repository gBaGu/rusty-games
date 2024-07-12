mod components;
mod error;
mod events;
mod game_info;
mod resources;
mod systems;

use bevy::prelude::*;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::{Game, GameState};

pub use components::Position;
pub use events::{
    CellUpdated, GameExit, GameOver, LocalGameTurn, NetworkGameTurn, StateUpdated, SuccessfulTurn,
};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::{Authority, CurrentGame, GameType, LocalGame};

use crate::grpc::NetworkSystems;
use resources::RefreshGameTimer;
use systems::*;

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub const BOARD_SIZE: usize = 3;

const BOARD_PROTO_SIZE: usize = BOARD_SIZE * BOARD_SIZE;
const PLAYERS_SIZE: usize = 2;

pub type TTTBoard = <TicTacToe as Game>::Board;

fn game_is_finished(game: Option<Res<CurrentGame>>) -> bool {
    matches!(
        game.and_then(|game| Some(game.state())),
        Some(GameState::Finished(_))
    )
}

fn game_is_in_progress(game: Option<Res<CurrentGame>>) -> bool {
    matches!(
        game.and_then(|game| Some(game.state())),
        Some(GameState::Turn(_))
    )
}

fn is_local_game(game: Option<Res<CurrentGame>>, local: Option<Res<LocalGame>>) -> bool {
    matches!(
        game.and_then(|game| Some(game.game_type())),
        Some(GameType::Local)
    ) && local.is_some()
}

fn is_network_game(game: Option<Res<CurrentGame>>) -> bool {
    matches!(
        game.and_then(|game| Some(game.game_type())),
        Some(GameType::Network(_))
    )
}

/// System set that is being run if there is an active game.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct GameSystems;

/// System set that is being run depending on a current game [`GameState`]
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameStateSystems {
    InProgress,
    Finished,
}

/// System set that is being run depending on a current game [`GameType`]
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GameTypeSystems {
    Local,
    Network,
}

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateUpdated>()
            .add_event::<CellUpdated>()
            .add_event::<NetworkGameTurn>()
            .add_event::<LocalGameTurn>()
            .add_event::<SuccessfulTurn>()
            .add_event::<GameOver>()
            .add_event::<GameExit>()
            .init_resource::<RefreshGameTimer>()
            .configure_sets(Update, GameSystems.run_if(resource_exists::<CurrentGame>))
            .configure_sets(
                Update,
                (
                    GameStateSystems::InProgress.run_if(game_is_in_progress),
                    GameStateSystems::Finished.run_if(game_is_finished),
                ),
            )
            .configure_sets(
                Update,
                (
                    GameTypeSystems::Local.run_if(is_local_game),
                    GameTypeSystems::Network.run_if(is_network_game),
                ),
            )
            .add_systems(
                Update,
                (
                    game_initialized.run_if(resource_added::<CurrentGame>),
                    handle_state_updated,
                )
                    .in_set(GameSystems),
            )
            .add_systems(
                Update,
                (
                    (
                        handle_get_game,
                        handle_make_turn,
                        (send_get_game, make_turn_network).in_set(NetworkSystems),
                    )
                        .in_set(GameTypeSystems::Network),
                    make_turn_local.in_set(GameTypeSystems::Local),
                )
                    .in_set(GameStateSystems::InProgress),
            );
    }
}
