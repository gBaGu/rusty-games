mod components;
mod resources;
mod systems;

use bevy::prelude::*;

use super::{LocalGame, PlayerActionInitialized, TTTBoard};
use components::Delay;
use systems::*;

pub use components::{BotBundle, NoDifficultyBotBundle, Strategy};
pub use resources::QLearningModel;

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(QLearningModel::load())
            .add_systems(Update, (start_delay, delay, initialize_action));
    }
}
