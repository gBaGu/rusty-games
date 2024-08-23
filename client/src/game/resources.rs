use std::marker::PhantomData;

use bevy::prelude::*;

use super::GAME_REFRESH_INTERVAL_SEC;

#[derive(Debug, Resource)]
pub struct GameMenuContext<T>(PhantomData<T>);

impl<T> Default for GameMenuContext<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

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
