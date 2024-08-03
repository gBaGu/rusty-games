mod components;
mod resources;
mod systems;

use bevy::prelude::*;

pub use components::{BotBundle, Strategy};

use super::{LocalGame, PendingAction, PlayerActionInitialized, TTTBoard};
use components::Delay;
use resources::QLearningModel;
use systems::*;

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(QLearningModel::load())
            .add_systems(Update, (start_delay, delay, initialize_action));
    }
}
