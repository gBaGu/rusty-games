mod components;
mod systems;

use bevy::prelude::*;

use super::PlayerActionApplied;
use crate::interface::common::PRIMARY_COLOR;
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
            Update,
            (
                create,
                update_player_info_border,
                update_next_player,
                set_winner,
                set_draw,
                action_sound,
            ),
        );
    }
}
