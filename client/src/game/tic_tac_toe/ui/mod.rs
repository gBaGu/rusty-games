mod components;
mod systems;

use bevy::prelude::*;
use game_server::game::tic_tac_toe::TicTacToe;

use super::PlayerActionApplied;
use crate::app_state::{AppState, MenuState};
use crate::interface::common::PRIMARY_COLOR;
use crate::interface::{enter_game_page, remove_game_page_context};
use components::{
    GameStateBox, GameStateInfoBundle, NextPlayer, NextPlayerImageBundle, PlayerImageBundle,
    PlayerInfoBundle,
};
use systems::*;

pub const FONT_SIZE: f32 = 40.0;
pub const ITEM_HEIGHT: f32 = 80.0;
pub const FRIENDLY_COLOR: Color = PRIMARY_COLOR;
pub const ENEMY_COLOR: Color = Color::rgb(0.94, 0.64, 0.64);

pub struct InGameUIPlugin;

impl Plugin for InGameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnExit(AppState::Menu(MenuState::Game)),
            remove_game_page_context::<TicTacToe>,
        )
        .add_systems(
            OnExit(AppState::Menu(MenuState::PlayAgainstBot)),
            remove_game_page_context::<TicTacToe>,
        )
        .add_systems(
            OnExit(AppState::Menu(MenuState::PlayOverNetwork)),
            remove_game_page_context::<TicTacToe>,
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
                enter_game_page::<TicTacToe>,
            ),
        );
    }
}
