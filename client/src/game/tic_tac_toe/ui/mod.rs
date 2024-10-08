mod components;
mod systems;

use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;

use super::PlayerActionApplied;
use crate::app_state::{AppState, MenuState};
use crate::interface;
use components::{
    GameStateBox, GameStateInfoBundle, NextPlayer, NextPlayerImageBundle, PlayerImageBundle,
    PlayerInfoBundle,
};
use systems::*;

pub const FONT_SIZE: f32 = 40.0;
pub const ITEM_HEIGHT: f32 = 80.0;
pub const FRIENDLY_COLOR: Color = interface::common::PRIMARY_COLOR;
pub const ENEMY_COLOR: Color = Color::srgb(0.94, 0.64, 0.64);

pub struct InGameUIPlugin;

impl Plugin for InGameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnExit(AppState::Menu(MenuState::Game)),
            interface::remove_game_page_context::<TicTacToe>,
        )
        .add_systems(
            OnExit(AppState::Menu(MenuState::PlayAgainstBot)),
            interface::remove_game_page_context::<TicTacToe>,
        )
        .add_systems(
            OnExit(AppState::Menu(MenuState::PlayOverNetwork)),
            interface::remove_game_page_context::<TicTacToe>,
        )
        .add_systems(
            Update,
            (
                create,
                update_player_info_border,
                update_next_player,
                set_winner,
                set_draw,
                action_sound,
                interface::enter_game_page::<TicTacToe>,
            ),
        );
    }
}
