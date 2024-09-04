use bevy::prelude::*;

use crate::game::GameInfo;

/// Event that indicates that the game is ready to be played.
/// Contains game entity.
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

/// Event that indicates that all gameplay and visual processes are finished
/// and game is ready to be closed.
/// Contains game entity.
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

/// Event that indicates that the app had left `AppState::Game` and
/// game visuals has been despawned.
/// Contains game entity.
#[derive(Clone, Copy, Debug, Deref, DerefMut, Event)]
pub struct GameLeft(Entity);

impl GameLeft {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

/// Event that indicates that the list of games this player can play is ready.
#[derive(Debug, Event)]
pub struct PlayerGamesReady {
    pub games: Result<Vec<GameInfo>, String>,
}

/// Event that indicates that submit button is pressed.
#[derive(Debug, Event)]
pub struct SubmitPressed {
    pub source: Entity,
}

/// Event that indicates that the button related to a particular setting option os pressed.
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

/// Event that indicates that join game button is pressed.
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
