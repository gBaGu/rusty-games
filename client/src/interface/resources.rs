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
