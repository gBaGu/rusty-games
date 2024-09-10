use bevy::prelude::*;

use crate::game::tic_tac_toe::board::{WIN_ANIMATION_SPRITE_COUNT, WIN_ANIMATION_SPRITE_SIZE};

/// Resource that holds information about how to break up a win animation spritesheet.
#[derive(Debug, Deref, Resource)]
pub struct WinAnimationSpriteSheet(Handle<TextureAtlasLayout>);

impl FromWorld for WinAnimationSpriteSheet {
    fn from_world(world: &mut World) -> Self {
        let texture_atlas = TextureAtlasLayout::from_grid(
            WIN_ANIMATION_SPRITE_SIZE.as_uvec2(),
            WIN_ANIMATION_SPRITE_COUNT as u32,
            1,
            None,
            None,
        );

        let mut texture_atlases = world
            .get_resource_mut::<Assets<TextureAtlasLayout>>()
            .unwrap();
        let texture_atlas_handle = texture_atlases.add(texture_atlas);
        Self(texture_atlas_handle)
    }
}
