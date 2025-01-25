use std::fmt;
use std::fmt::Formatter;
use std::time::Duration;

use bevy::prelude::*;
use game_server::core;

use crate::game::{BotAuthority, BotDifficulty, PlayerPosition};

/// Component that creates human-like delay before the bot will perform its action.
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

/// Component that defines the logic that will be used to choose bot action.
#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub enum Strategy {
    Random,
    QLearning,
}

impl fmt::Display for Strategy {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
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
    pub fn new(id: u64, player_position: core::PlayerPosition, strategy: Strategy) -> Self {
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
        player_position: core::PlayerPosition,
        strategy: Strategy,
        difficulty: BotDifficulty,
    ) -> Self {
        Self {
            inner: NoDifficultyBotBundle::new(id, player_position, strategy),
            difficulty,
        }
    }
}
