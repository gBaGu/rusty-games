use bevy::prelude::{Deref, DerefMut, Resource, Timer, TimerMode};

use crate::interface::common::GAME_LIST_REFRESH_INTERVAL_SEC;

#[derive(Deref, DerefMut, Resource)]
pub struct RefreshGamesTimer(pub Timer);

impl Default for RefreshGamesTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_LIST_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Debug, Default, Resource)]
pub struct Settings {
    user_id: Option<u64>,
}

impl Settings {
    pub fn user_id(&self) -> Option<u64> {
        self.user_id
    }

    pub fn set_user_id(&mut self, value: u64) {
        self.user_id = Some(value);
    }
}
