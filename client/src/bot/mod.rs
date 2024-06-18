mod components;
mod strategy;
mod systems;

use bevy::prelude::*;

pub use components::Bot;
pub use strategy::MoveStrategy;

use crate::game::GameStateSystems;
use systems::make_turn;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct BotSystems;

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            make_turn
                .in_set(BotSystems)
                .in_set(GameStateSystems::InProgress),
        );
    }
}
