use bevy::asset::{AssetPath, AssetServer};
use bevy::audio::AudioPlayer;
use bevy::ecs::system::{Commands, EntityCommands};

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
        self.spawn(AudioPlayer::new(asset_server.load(path)))
    }
}
