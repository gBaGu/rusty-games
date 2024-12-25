mod board;
mod bot;
mod components;
mod menu;
mod resources;
mod systems;
mod ui;

use bevy::prelude::*;
use game_server::core;
use game_server::core::tic_tac_toe::TicTacToe;

use resources::Images;
use systems::*;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub type TTTBoard = <TicTacToe as core::Game>::Board;
pub type Action = <TicTacToe as core::Game>::TurnData;

type LocalGame = super::LocalGame<TicTacToe>;
type LocalGameBundle = super::LocalGameBundle<TicTacToe, Action>;
type NetworkGameBundle = super::NetworkGameBundle<TicTacToe, Action>;
type PendingAction = super::PendingAction<Action>;
type PendingActionQueue = super::PendingActionQueue<Action>;
type PlayerActionInitialized = super::PlayerActionInitialized<Action>;
type PlayerActionApplied = super::PlayerActionApplied<Action>;

pub struct TicTacToePlugin;

impl Plugin for TicTacToePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            board::BoardPlugin,
            bot::BotPlugin,
            menu::GameMenuPlugin,
            ui::InGameUIPlugin,
        ))
        .add_event::<PlayerActionInitialized>()
        .add_event::<PlayerActionApplied>()
        .add_systems(Startup, init)
        .add_systems(
            Update,
            (
                handle_create_game_reply,
                handle_get_game_reply_on_join,
                handle_get_game,
                create,
                create_pending_action,
                apply_confirmed,
            ),
        );
    }
}
