use bevy::prelude::{Entity, Event};

use crate::game::GameInfo;

#[derive(Debug, Event)]
pub struct SubmitPressed {
    pub source: Entity,
}

#[derive(Debug, Event)]
pub struct PlayerGamesReady {
    pub games: Result<Vec<GameInfo>, String>,
}
