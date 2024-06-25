mod components;
mod events;
mod systems;

use std::time::Duration;

use bevy::prelude::*;

pub use components::{Board, BoardBundle};
pub use events::{TileFilled, TilePressed};

use crate::board::components::WinAnimationBundle;
use crate::game::{GameStateSystems, GameSystems};
use systems::{create, handle_game_over, handle_input, set_tile_image, win_animation};

pub const BORDER_WIDTH: f32 = 1.0;
pub const WIN_ANIMATION_PATH: &str = "sprites/win_animation.png";
pub const WIN_ANIMATION_SPRITE_COUNT: usize = 20;
pub const WIN_ANIMATION_SPRITE_SIZE: Vec2 = Vec2::new(51.0, 150.0);

/// Returns center coordinates for a board tile with given `pos`.
pub fn calculate_tile_center(board_size: Vec2, tile_size: Vec2, tile_pos: (u32, u32)) -> Vec2 {
    let tile_x =
        (tile_size.x + BORDER_WIDTH) * tile_pos.1 as f32 + tile_size.x / 2.0 - board_size.x / 2.0;
    let tile_y =
        (tile_size.y + BORDER_WIDTH) * (2 - tile_pos.0) as f32 + tile_size.y / 2.0 - board_size.y / 2.0;
    Vec2::new(tile_x, tile_y)
}

/// Returns tile size for a given board size.
pub fn calculate_tile_size(board_size: Vec2) -> Vec2 {
    let tile_width = (board_size.x - BORDER_WIDTH * 2.0) / 3.0;
    let tile_height = (board_size.y - BORDER_WIDTH * 2.0) / 3.0;
    Vec2::new(tile_width, tile_height)
}

/// Returns [`WinAnimationBundle`] that will be placed on top of `from` and `to` tiles.
pub fn create_win_animation(
    asset_server: &AssetServer,
    texture_atlas_layouts: &mut Assets<TextureAtlasLayout>,
    board_size: Vec2,
    from: (u32, u32),
    to: (u32, u32),
) -> WinAnimationBundle {
    let texture = asset_server.load(WIN_ANIMATION_PATH);
    let layout = TextureAtlasLayout::from_grid(
        WIN_ANIMATION_SPRITE_SIZE,
        WIN_ANIMATION_SPRITE_COUNT,
        1,
        None,
        None,
    );
    let texture_atlas_layout = texture_atlas_layouts.add(layout);

    let tile_size = calculate_tile_size(board_size);
    let from_center = calculate_tile_center(board_size, tile_size, from);
    let to_center = calculate_tile_center(board_size, tile_size, to);

    let center = (from_center + to_center) / 2.0;
    let mut transform = Transform::from_translation(center.extend(1.0));

    let line_vector = (from_center - center).normalize();
    transform.rotation = Quat::from_rotation_arc(Vec3::Y, line_vector.extend(0.));

    let target_length = from_center.distance(to_center) + tile_size.x.min(tile_size.y);
    transform.scale = Vec2::splat(target_length / WIN_ANIMATION_SPRITE_SIZE.y).extend(1.);

    WinAnimationBundle::new(
        WIN_ANIMATION_SPRITE_COUNT - 1,
        Duration::from_millis(100),
        texture,
        texture_atlas_layout,
        transform,
    )
}

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilePressed>()
            .add_event::<TileFilled>()
            .add_systems(
                Update,
                (
                    create,
                    set_tile_image.in_set(GameSystems),
                    handle_input.in_set(GameStateSystems::InProgress),
                    (handle_game_over, win_animation).in_set(GameStateSystems::Finished),
                ),
            );
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_calculate_tile_center() {
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (0, 0)),
            Vec2::new(-35.0, 27.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (0, 1)),
            Vec2::new(-4.0, 27.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (0, 2)),
            Vec2::new(27.0, 27.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (1, 0)),
            Vec2::new(-35.0, -4.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (1, 1)),
            Vec2::new(-4.0, -4.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (1, 2)),
            Vec2::new(27.0, -4.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (2, 0)),
            Vec2::new(-35.0, -35.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (2, 1)),
            Vec2::new(-4.0, -35.0)
        );
        assert_eq!(
            calculate_tile_center(Vec2::splat(100.0), Vec2::splat(30.0), (2, 2)),
            Vec2::new(27.0, -35.0)
        );
    }

    #[test]
    fn test_calculate_tile_size() {
        assert_eq!(calculate_tile_size(Vec2::splat(302.0)), Vec2::splat(100.0));
        assert_eq!(calculate_tile_size(Vec2::new(62.0, 32.0)), Vec2::new(20.0, 10.0));
        assert_eq!(calculate_tile_size(Vec2::new(50.0, 53.0)), Vec2::new(16.0, 17.0));
    }
}
