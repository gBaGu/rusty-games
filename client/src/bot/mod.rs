mod components;
mod systems;
mod strategy;

use bevy::prelude::*;

pub use components::Bot;
pub use strategy::MoveStrategy;
use crate::app_state::AppState;
use crate::bot::systems::make_turn;

pub struct BotPlugin;

impl Plugin for BotPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, make_turn.run_if(in_state(AppState::Game)));
    }
}