mod components;
mod systems;

use bevy::prelude::*;

pub use components::InGameUIBundle;

use super::common::PRIMARY_COLOR;
use crate::app_state::AppState;
use systems::*;

pub const FONT_SIZE: f32 = 40.0;
pub const ITEM_HEIGHT: f32 = 80.0;
pub const PLAYER_TURN_COLOR: Color = PRIMARY_COLOR;
pub const ENEMY_TURN_COLOR: Color = Color::rgb(0.94, 0.64, 0.64);

pub struct InGameUIPlugin;

impl Plugin for InGameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                create,
                handle_state_update.after(create),
                handle_end_turn
                    .run_if(in_state(AppState::Game).or_else(in_state(AppState::Paused))),
            ),
        );
    }
}
