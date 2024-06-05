use std::time::Duration;
use bevy::prelude::{Component, Timer, TimerMode};

#[derive(Debug, Component)]
pub struct Bot {
    id: u64,
    timer: Timer,
}

impl Bot {
    pub fn new(id: u64) -> Self {
        let mut timer = Timer::new(Duration::default(), TimerMode::Once);
        timer.pause();
        Self {
            id,
            timer,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }

    pub fn timer_mut(&mut self) -> &mut Timer {
        &mut self.timer
    }

    pub fn reset_timer(&mut self) {
        self.timer.reset();
        self.timer.pause();
    }

    pub fn start_timer(&mut self, duration: Duration) {
        self.timer.set_duration(duration);
        self.timer.unpause();
    }
}
