use bevy::prelude::*;

/// Structures setting values required to create new game with bot.
#[derive(Debug, Component)]
pub struct BotGameSettings {
    strategy: Entity,
    difficulty: Entity,
}

impl BotGameSettings {
    pub fn new(strategy: Entity, difficulty: Entity) -> Self {
        Self {
            strategy,
            difficulty,
        }
    }

    pub fn strategy(&self) -> Entity {
        self.strategy
    }

    pub fn difficulty(&self) -> Entity {
        self.difficulty
    }
}

/// Structures setting values required to create new network game.
#[derive(Debug, Component)]
pub struct NetworkGameSettings {
    opponent: Entity,
}

impl NetworkGameSettings {
    pub fn new(opponent: Entity) -> Self {
        Self { opponent }
    }

    pub fn opponent(&self) -> Entity {
        self.opponent
    }
}
