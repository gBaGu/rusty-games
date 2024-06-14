use std::time::Duration;
use bevy::prelude::{Component, Timer, TimerMode};

#[derive(Debug, Component)]
pub struct Bot {
    id: u64,
    delay_timer: Timer,
}

impl Bot {
    pub fn new(id: u64) -> Self {
        let mut timer = Timer::new(Duration::default(), TimerMode::Once);
        timer.pause();
        Self {
            id,
            delay_timer: timer,
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn waiting_delay(&self) -> bool {
        !self.delay_timer.paused()
    }

    pub fn start_delay(&mut self, duration: Duration) {
        self.delay_timer.set_duration(duration);
        self.delay_timer.unpause();
    }

    pub fn tick_delay(&mut self, delta: Duration) -> &Timer {
        self.delay_timer.tick(delta)
    }

    pub fn reset_delay(&mut self) {
        self.delay_timer.reset();
        self.delay_timer.pause();
    }
}
