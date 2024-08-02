use bevy::prelude::*;

use crate::game::GameInfo;

#[derive(Debug, Deref, DerefMut, Event)]
pub struct GameReady(Entity);

impl GameReady {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut, Event)]
pub struct GameReadyToExit(Entity);

impl GameReadyToExit {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut, Event)]
pub struct GameExit(Entity);

impl GameExit {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

#[derive(Debug, Event)]
pub struct SubmitPressed {
    pub source: Entity,
}

#[derive(Debug, Event)]
pub struct PlayerGamesReady {
    pub games: Result<Vec<GameInfo>, String>,
}
