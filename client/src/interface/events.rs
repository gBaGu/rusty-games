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
pub struct PlayerGamesReady {
    pub games: Result<Vec<GameInfo>, String>,
}

#[derive(Debug, Event)]
pub struct SubmitPressed {
    pub source: Entity,
}

#[derive(Debug, Event)]
pub struct SettingOptionPressed {
    pub source: Entity,
    pub settings: Entity,
}

impl SettingOptionPressed {
    pub fn new(source: Entity, settings: Entity) -> Self {
        Self {
            source,
            settings,
        }
    }
}

#[derive(Debug, Event)]
pub struct JoinPressed {
    game_id: u64,
}

impl JoinPressed {
    pub fn new(game_id: u64) -> Self {
        Self { game_id }
    }

    pub fn game_id(&self) -> u64 {
        self.game_id
    }
}
