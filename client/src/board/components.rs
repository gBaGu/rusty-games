use bevy::prelude::*;
use std::time::Duration;

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

#[derive(Component)]
pub struct WinAnimation;

#[derive(Component)]
pub struct OneTimeAnimation {
    last_sprite_index: usize,
    timer: Timer,
}

impl OneTimeAnimation {
    pub fn new(last_sprite_index: usize, timer: Timer) -> Self {
        Self {
            last_sprite_index,
            timer,
        }
    }

    pub fn last_sprite_index(&self) -> usize {
        self.last_sprite_index
    }

    pub fn tick(&mut self, delta: Duration) -> &Timer {
        self.timer.tick(delta)
    }
}

#[derive(Bundle)]
pub struct WinAnimationBundle {
    pub animation: OneTimeAnimation,
    pub sprite_sheet: SpriteSheetBundle,
    pub tag: WinAnimation,
}

impl WinAnimationBundle {
    pub fn new(
        last_sprite_index: usize,
        transition_duration: Duration,
        texture: Handle<Image>,
        layout: Handle<TextureAtlasLayout>,
        transform: Transform,
    ) -> Self {
        Self {
            animation: OneTimeAnimation::new(
                last_sprite_index,
                Timer::new(transition_duration, TimerMode::Repeating),
            ),
            sprite_sheet: SpriteSheetBundle {
                texture,
                atlas: TextureAtlas { layout, index: 0 },
                transform,
                ..default()
            },
            tag: WinAnimation,
        }
    }
}
