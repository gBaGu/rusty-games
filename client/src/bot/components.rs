use bevy::prelude::{Component, Timer, TimerMode};
use rand::{thread_rng, Rng};
use std::time::Duration;

use crate::bot::strategy::MoveStrategy;
use crate::game::{Position, TTTBoard};

const MIN_MOVE_DELAY: u64 = 500;
const MAX_MOVE_DELAY: u64 = 1500;

#[derive(Component)]
pub struct Bot {
    id: u64,
    delay_timer: Timer,
    move_strategy: MoveStrategy,
}

impl Bot {
    pub fn new(id: u64, move_strategy: MoveStrategy) -> Self {
        let mut timer = Timer::new(Duration::default(), TimerMode::Once);
        timer.pause();
        Self {
            id,
            delay_timer: timer,
            move_strategy,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn waiting_delay(&self) -> bool {
        !self.delay_timer.paused()
    }

    pub fn start_delay(&mut self) {
        let mut rng = thread_rng();
        let milliseconds = rng.gen_range(MIN_MOVE_DELAY..MAX_MOVE_DELAY);
        self.delay_timer
            .set_duration(Duration::from_millis(milliseconds));
        self.delay_timer.unpause();
    }

    pub fn tick_delay(&mut self, delta: Duration) -> &Timer {
        self.delay_timer.tick(delta)
    }

    pub fn reset_delay(&mut self) {
        self.delay_timer.reset();
        self.delay_timer.pause();
    }

    pub fn get_move(&self, board: &TTTBoard) -> Option<Position> {
        self.move_strategy.get_move(board)
    }
}
