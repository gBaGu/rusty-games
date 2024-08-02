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
    Random(RandomStrategy),
    QLearning(QLearningStrategy),
}

#[derive(Clone, Copy, Debug, Component)]
pub struct RandomStrategy;

#[derive(Clone, Copy, Debug, Component)]
pub struct QLearningStrategy {
    difficulty: BotDifficulty,
}

impl QLearningStrategy {
    pub fn new(difficulty: BotDifficulty) -> Self {
        Self { difficulty }
    }

    pub fn difficulty(&self) -> BotDifficulty {
        self.difficulty
    }
}

#[derive(Debug, Bundle)]
pub struct BotBundle<T: Component + Send + Sync + 'static> {
    player: PlayerPosition,
    auth: BotAuthority,
    strategy: T,
    delay: Delay,
}

impl<T: Component + Send + Sync + 'static> BotBundle<T> {
    pub fn new(id: u64, player_position: PlayerId, strategy: T) -> Self {
        Self {
            player: PlayerPosition::new(player_position),
            auth: BotAuthority::new(id),
            strategy,
            delay: Delay::default(),
        }
    }
}
