mod components;
mod error;
mod events;
mod game_info;
mod resources;
mod systems;
mod tic_tac_toe;

use bevy::prelude::*;
use game_server::game;
use game_server::game::tic_tac_toe::TicTacToe;

pub use components::{
    ActiveGame, Board, BotDifficulty, CurrentPlayer, CurrentUser, GameLink, LocalGame, LocalGameBundle,
    NetworkGame, NetworkGameBundle, PendingExistingGameBundle, PendingNewGameBundle,
    PlayerPosition, UserAuthority, Winner,
};
pub use events::{BotReady, CreateGame, Draw, PlayerWon, StateUpdated, TurnStart};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::GameMenuContext;
pub use tic_tac_toe::bot::Strategy as TTTBotStrategy;

use components::{
    BotAuthority, CurrentUserPlayerBundle, NetworkPlayerBundle, PendingAction, PendingActionBundle,
};
use events::{PlayerActionApplied, PlayerActionInitialized};
use resources::RefreshGameTimer;
use systems::*;

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const BOARD_SIZE: usize = 3;

const BOARD_PROTO_SIZE: usize = BOARD_SIZE * BOARD_SIZE;
const PLAYERS_SIZE: usize = 2;

pub type TTTBoard = <TicTacToe as game::Game>::Board;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tic_tac_toe::TicTacToePlugin)
            .add_event::<CreateGame>()
            .add_event::<BotReady>()
            .add_event::<StateUpdated>()
            .add_event::<TurnStart>()
            .add_event::<Draw>()
            .add_event::<PlayerWon>()
            .init_resource::<RefreshGameTimer>()
            .add_systems(
                Update,
                (
                    handle_make_turn,
                    handle_state_updated,
                    update_current_player,
                    handle_draw,
                    handle_win,
                ),
            );
    }
}
