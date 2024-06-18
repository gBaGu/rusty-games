mod components;
mod systems;

use bevy::prelude::*;

pub use components::InGameUIBundle;

use super::common::PRIMARY_COLOR;
use crate::game::GameSystems;
use systems::{create, handle_state_update};

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
                handle_state_update.in_set(GameSystems).after(create),
            ),
        );
    }
}
