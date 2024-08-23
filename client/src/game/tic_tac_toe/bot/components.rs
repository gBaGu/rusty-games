use std::time::Duration;

use bevy::prelude::*;
use game_server::game::PlayerId;

use crate::game::{BotAuthority, BotDifficulty, PlayerPosition};

/// Delay before the bot will perform its action
#[derive(Debug, Component, Deref, DerefMut)]
pub struct Delay(Timer);

impl Default for Delay {
    fn default() -> Self {
        Self(Timer::new(Duration::default(), TimerMode::Once))
    }
}

impl Delay {
    pub fn start(&mut self, duration: Duration) {
        self.set_duration(duration);
        self.unpause();
    }

    pub fn reset(&mut self) {
        self.0.reset();
        self.pause();
    }
}

#[derive(Clone, Copy, Debug, Component)]
pub enum Strategy {
    Random,
    QLearning,
}

impl Strategy {
    pub fn has_difficulty(&self) -> bool {
        match self {
            Self::Random => false,
            Self::QLearning => true,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NoDifficultyBotBundle {
    player: PlayerPosition,
    auth: BotAuthority,
    strategy: Strategy,
    delay: Delay,
}

impl NoDifficultyBotBundle {
    pub fn new(id: u64, player_position: PlayerId, strategy: Strategy) -> Self {
        Self {
            player: PlayerPosition::new(player_position),
            auth: BotAuthority::new(id),
            strategy,
            delay: Delay::default(),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct BotBundle {
    inner: NoDifficultyBotBundle,
    difficulty: BotDifficulty,
}

impl BotBundle {
    pub fn new(
        id: u64,
        player_position: PlayerId,
        strategy: Strategy,
        difficulty: BotDifficulty,
    ) -> Self {
        Self {
            inner: NoDifficultyBotBundle::new(id, player_position, strategy),
            difficulty,
        }
    }
}
