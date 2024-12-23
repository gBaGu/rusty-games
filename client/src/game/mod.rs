mod components;
mod events;
mod game_info;
mod pending_action;
mod resources;
mod systems;
mod tic_tac_toe;

use bevy::prelude::*;
use game_server::core;
use game_server::core::tic_tac_toe::TicTacToe;

use crate::grpc::NetworkSystems;
use components::{
    BotAuthority, CurrentPlayer, CurrentUserPlayerBundle, LocalGame, LocalGameBundle, NetworkGame,
    NetworkGameBundle, NetworkPlayerBundle, PendingExistingGameBundle, PendingNewGameBundle,
    PlayerPosition, UserAuthority,
};
use events::{ActionConfirmationFailed, PlayerActionApplied, PlayerActionInitialized};
use resources::RefreshGameTimer;
use systems::*;

pub use components::{
    ActiveGame, Board, BotDifficulty, CurrentUser, GameLink, PendingActionQueue, Winner,
};
pub use events::{
    ServerActionReceived, BotReady, Draw, GameDataReady, PlayerWon, StateUpdated, TurnStart,
};
pub use game_info::{FullGameInfo, GameInfo};
pub use pending_action::PendingAction;
pub use resources::GameMenuContext;

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const BOARD_SIZE: usize = 3;

const BOARD_PROTO_SIZE: usize = BOARD_SIZE * BOARD_SIZE;
const PLAYERS_SIZE: usize = 2;

pub type TTTBoard = <TicTacToe as core::Game>::Board;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tic_tac_toe::TicTacToePlugin)
            .add_event::<ServerActionReceived<core::GridIndex>>()
            .add_event::<GameDataReady>()
            .add_event::<ActionConfirmationFailed>()
            .add_event::<BotReady>()
            .add_event::<StateUpdated>()
            .add_event::<TurnStart>()
            .add_event::<Draw>()
            .add_event::<PlayerWon>()
            .init_resource::<RefreshGameTimer>()
            .add_systems(
                Update,
                (
                    handle_local_game_creation,
                    handle_state_updated,
                    update_current_player,
                    handle_draw,
                    handle_win,
                    set_game_finished,
                    update_current_user,
                    clear_foreign_network_games,
                    clear_game_on_exit,
                    send_pending_action::<core::GridIndex>.in_set(NetworkSystems),
                    revert_action_status::<core::GridIndex>,
                    handle_game_session_update::<core::GridIndex>,
                    handle_action_from_server::<core::GridIndex>,
                    handle_game_session_finished,
                ),
            );
    }
}
