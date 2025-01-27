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

use crate::{grpc, interface, util};
use components::{
    BotAuthority, CurrentPlayer, CurrentUserPlayerBundle, LocalGame, LocalGameBundle,
    NetworkGameBundle, NetworkPlayerBundle, PendingExistingGameBundle, PendingNewGameBundle,
    PlayerPosition, UserAuthority,
};
use events::{
    ActionApplied, ActionConfirmationFailed, ActionConfirmed, ActionDropped, ActionEnqueued,
    ActionInitialized, ActionQueueNextChanged,
};
use pending_action::{ConfirmationStatus, PendingAction};
use resources::RefreshGameTimer;
use systems::*;

pub use components::{
    ActiveGame, Board, BotDifficulty, CurrentUser, GameLink, NetworkGame, PendingActionQueue,
    Winner,
};
pub use events::{
    BotReady, Draw, GameDataReady, GameEntityReady, PlayerWon, StateUpdated, TurnStart,
};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::GameMenuContext;

pub const ACTION_RESEND_INTERVAL_SEC: f32 = 1.0;
pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub const BOARD_SIZE: usize = 3;

const BOARD_PROTO_SIZE: usize = BOARD_SIZE * BOARD_SIZE;
const PLAYERS_SIZE: usize = 2;

pub type TTTBoard = <TicTacToe as core::Game>::Board;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(tic_tac_toe::TicTacToePlugin)
            .add_event::<GameDataReady>()
            .add_event::<GameEntityReady>()
            .add_event::<ActionQueueNextChanged>()
            .add_event::<ActionInitialized<core::GridIndex>>()
            .add_event::<ActionEnqueued<core::GridIndex>>()
            .add_event::<ActionConfirmationFailed<core::GridIndex>>()
            .add_event::<ActionConfirmed<core::GridIndex>>()
            .add_event::<ActionDropped<core::GridIndex>>()
            .add_event::<ActionApplied<core::GridIndex>>()
            .add_event::<BotReady>()
            .add_event::<StateUpdated>()
            .add_event::<TurnStart>()
            .add_event::<Draw>()
            .add_event::<PlayerWon>()
            .init_resource::<RefreshGameTimer>()
            .add_systems(
                Update,
                (
                    initialize_game_session::<TicTacToe>.in_set(grpc::NetworkSystems),
                    network_game_initialization_finished,
                    send_pending_action::<core::GridIndex>.in_set(grpc::NetworkSystems),
                    action_confirmation_failed::<core::GridIndex>,
                    handle_action_from_server::<core::GridIndex>,
                    close_session,
                ),
            )
            .add_systems(
                Update,
                (
                    handle_game_spawn,
                    handle_local_game_creation,
                    handle_state_updated,
                    update_current_player,
                    handle_draw,
                    handle_win,
                    set_game_finished,
                    update_current_user,
                    clear_foreign_network_games,
                    clear_game_on_exit,
                    create_pending_action::<core::GridIndex>,
                    confirm_local_game_action::<core::GridIndex>,
                    action_queue_next_changed::<core::GridIndex>,
                    create_resend_action_timer::<core::GridIndex>,
                    resend_action_timer_tick,
                    revert_action_status::<core::GridIndex>,
                    apply_confirmed::<TicTacToe>,
                ),
            )
            .add_systems(
                Update,
                (
                    log_enqueued_action::<core::GridIndex>,
                    log_confirmed_action::<core::GridIndex>,
                    log_dropped_action::<core::GridIndex>,
                ),
            )
            .add_systems(
                Update,
                (
                    interface::set_local_option_setting::<BotDifficulty>,
                    interface::update_option_buttons_border::<BotDifficulty>,
                ),
            );
        util::watched_value::setup::<u64>(app);
        util::watched_value::setup::<BotDifficulty>(app);
    }
}
