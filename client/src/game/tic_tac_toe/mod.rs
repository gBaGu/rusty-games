mod board;
mod bot;
mod components;
mod menu;
mod resources;
mod systems;
mod ui;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::Game;

use crate::grpc::NetworkSystems;
use resources::Images;
use systems::*;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub type TTTBoard = <TicTacToe as Game>::Board;

type LocalGame = super::LocalGame<TicTacToe>;
type LocalGameBundle = super::LocalGameBundle<TicTacToe>;
type NetworkGameBundle = super::NetworkGameBundle<TicTacToe>;
type PendingAction = super::PendingAction<GridIndex>;
type PendingActionBundle = super::PendingActionBundle<GridIndex>;
type PlayerActionInitialized = super::PlayerActionInitialized<GridIndex>;
type PlayerActionApplied = super::PlayerActionApplied<GridIndex>;

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
                apply_action,
                (send_pending_action, send_get_game).in_set(NetworkSystems),
            ),
        );
    }
}
