use bevy::prelude::*;

use crate::game::Position;
use crate::interface::common::SECONDARY_COLOR;

/// Empty component to indicate that an entity is a board.
#[derive(Component)]
pub struct Board;

// Bundles

/// Bundle for a board.
/// Contains [`SpriteBundle`] and a [`Board`].
#[derive(Bundle)]
pub struct BoardBundle {
    pub background: SpriteBundle,
    pub board: Board,
}

impl BoardBundle {
    pub fn new(size: Vec2, translation: Vec3) -> Self {
        Self {
            background: SpriteBundle {
                sprite: Sprite {
                    color: SECONDARY_COLOR,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(translation),
                ..default()
            },
            board: Board,
        }
    }
}

/// Bundle for a board tile.
/// Contains [`SpriteBundle`] and a [`Position`].
#[derive(Bundle)]
pub struct TileBundle {
    pub sprite: SpriteBundle,
    pub position: Position,
}

impl TileBundle {
    pub fn new(size: Vec2, translation: Vec3, position: Position) -> Self {
        Self {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::NONE,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(translation),
                ..default()
            },
            position,
        }
    }
}
