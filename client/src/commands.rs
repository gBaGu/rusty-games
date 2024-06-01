use bevy::asset::{AssetPath, AssetServer};
use bevy::audio::AudioBundle;
use bevy::ecs::system::EntityCommands;
use bevy::prelude::{default, Commands, Bundle, BuildChildren};

pub trait CommandsExt {
    fn play_sound<'a>(
        &mut self,
        asset_server: &AssetServer,
        path: impl Into<AssetPath<'a>>,
    ) -> EntityCommands;
}

impl CommandsExt for Commands<'_, '_> {
    fn play_sound<'a>(
        &mut self,
        asset_server: &AssetServer,
        path: impl Into<AssetPath<'a>>,
    ) -> EntityCommands {
        self.spawn(AudioBundle {
            source: asset_server.load(path),
            ..default()
        })
    }
}

pub trait EntityCommandsExt {
    fn with_child(&mut self, bundle: impl Bundle) -> &mut Self;
}

impl EntityCommandsExt for EntityCommands<'_> {
    fn with_child(&mut self, bundle: impl Bundle) -> &mut Self {
        self.with_children(|builder| {
            builder.spawn(bundle);
        })
    }
}


