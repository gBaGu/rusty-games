mod components;
mod events;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy::ui::UiSystem;
use game_server::core;

pub use events::TilePressed;

use super::{LocalGame, PlayerActionApplied};
use crate::app_state::AppState;
use components::{TileBundle, WinAnimation, WinAnimationBundle};
use resources::WinAnimationSpriteSheet;
use systems::*;

pub const BORDER_WIDTH: f32 = 1.0;
pub const WIN_ANIMATION_PATH: &str = "sprites/win_animation.png";
pub const WIN_ANIMATION_SPRITE_COUNT: usize = 20;
pub const WIN_ANIMATION_TRANSITION_INTERVAL: u64 = 50;
pub const WIN_ANIMATION_SPRITE_SIZE: Vec2 = Vec2::new(51.0, 150.0);

/// Returns center coordinates for a board tile with given `pos`.
pub fn calculate_tile_center(board_size: Vec2, tile_size: Vec2, tile_pos: core::GridIndex) -> Vec2 {
    let tile_x = (tile_size.x + BORDER_WIDTH) * tile_pos.col() as f32 + tile_size.x / 2.0
        - board_size.x / 2.0;
    let tile_y = (tile_size.y + BORDER_WIDTH) * (2 - tile_pos.row()) as f32 + tile_size.y / 2.0
        - board_size.y / 2.0;
    Vec2::new(tile_x, tile_y)
}

/// Returns tile size for a given board size.
pub fn calculate_tile_size(board_size: Vec2) -> Vec2 {
    let tile_width = (board_size.x - BORDER_WIDTH * 2.0) / 3.0;
    let tile_height = (board_size.y - BORDER_WIDTH * 2.0) / 3.0;
    Vec2::new(tile_width, tile_height)
}

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WinAnimationSpriteSheet>()
            .add_event::<TilePressed>()
            .add_systems(
                Update,
                (
                    handle_mouse_input.run_if(in_state(AppState::Game)),
                    initialize_action,
                    set_tile_image,
                    create_win_animation,
                    update_win_animation,
                ),
            )
            .add_systems(PostUpdate, create.after(UiSystem::Layout));
    }
}
