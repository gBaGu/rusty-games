use bevy::prelude::*;
use smallvec::SmallVec;

use crate::game::BotDifficulty;

/// Event that signals that strategy setting was updated.
/// Stores setting entity and [`BotDifficulty`] options available for new strategy.
#[derive(Debug, Event)]
pub struct StrategyUpdated {
    setting: Entity,
    difficulty_options: SmallVec<[BotDifficulty; 3]>,
}

impl StrategyUpdated {
    pub fn new(
        setting: Entity,
        difficulty_options: SmallVec<[BotDifficulty; 3]>,
    ) -> Self {
        Self {
            setting,
            difficulty_options,
        }
    }

    pub fn setting(&self) -> Entity {
        self.setting
    }

    pub fn difficulty_supported(&self, difficulty: &BotDifficulty) -> bool {
        self.difficulty_options.contains(difficulty)
    }
}
