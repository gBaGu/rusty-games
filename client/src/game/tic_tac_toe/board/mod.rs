mod components;
mod events;
mod resources;
mod systems;

use bevy::prelude::*;
use bevy::ui::UiSystem;

use super::{LocalGame, PlayerActionApplied};
use crate::app_state::AppState;
use components::{TileBundle, WinAnimation, WinAnimationBundle};
use resources::WinAnimationSpriteSheet;
use systems::*;

pub use events::TilePressed;

pub const BORDER_WIDTH: f32 = 1.0;
pub const WIN_ANIMATION_PATH: &str = "sprites/win_animation.png";
pub const WIN_ANIMATION_SPRITE_COUNT: usize = 20;
pub const WIN_ANIMATION_TRANSITION_INTERVAL: u64 = 50;
pub const WIN_ANIMATION_SPRITE_SIZE: Vec2 = Vec2::new(51.0, 150.0);

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
