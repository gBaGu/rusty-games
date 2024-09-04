use std::time::Duration;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;

#[derive(Component)]
pub struct Border;

#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct Tile(GridIndex);

impl From<GridIndex> for Tile {
    fn from(value: GridIndex) -> Self {
        Self(value)
    }
}

impl From<Tile> for GridIndex {
    fn from(value: Tile) -> Self {
        value.0
    }
}

#[derive(Component)]
pub struct WinAnimation {
    last_sprite_index: usize,
    timer: Timer,
}

impl WinAnimation {
    pub fn new(last_sprite_index: usize, transition_duration: Duration) -> Self {
        Self {
            last_sprite_index,
            timer: Timer::new(transition_duration, TimerMode::Repeating),
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
pub struct BorderBundle {
    sprite: SpriteBundle,
    border: Border,
}

impl BorderBundle {
    pub fn new(color: Color, size: Vec2, translation: Vec3) -> Self {
        Self {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(translation),
                ..default()
            },
            border: Border,
        }
    }
}

/// Bundle for a board tile.
/// Contains [`SpriteBundle`] and a [`Tile`].
#[derive(Bundle)]
pub struct TileBundle {
    sprite: SpriteBundle,
    tile: Tile,
}

impl TileBundle {
    pub fn new(size: Vec2, translation: Vec3, position: GridIndex, img: Handle<Image>) -> Self {
        Self {
            sprite: SpriteBundle {
                sprite: Sprite {
                    color: Color::NONE,
                    custom_size: Some(size),
                    ..default()
                },
                transform: Transform::from_translation(translation),
                texture: img,
                ..default()
            },
            tile: position.into(),
        }
    }

    pub fn new_empty(size: Vec2, translation: Vec3, position: GridIndex) -> Self {
        Self::new(size, translation, position, Handle::default())
    }
}

/// A bundle for drawing win animation
#[derive(Bundle)]
pub struct WinAnimationBundle {
    pub animation: WinAnimation,
    pub sprite: SpriteBundle,
    pub atlas: TextureAtlas,
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
            animation: WinAnimation::new(last_sprite_index, transition_duration),
            sprite: SpriteBundle {
                texture,
                transform,
                ..default()
            },
            atlas: TextureAtlas { layout, index: 0 },
        }
    }
}
