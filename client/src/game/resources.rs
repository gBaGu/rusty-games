use bevy::prelude::*;

use super::GAME_REFRESH_INTERVAL_SEC;

#[derive(Deref, DerefMut, Resource)]
pub struct RefreshGameTimer(pub Timer);

impl Default for RefreshGameTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}
