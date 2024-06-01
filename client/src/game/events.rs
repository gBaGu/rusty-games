use bevy::prelude::{Deref, Event};
use game_server::game::{FinishedState, GameState};

use super::Position;

#[derive(Debug, Deref, Event)]
pub struct StateUpdated(pub GameState);

#[derive(Debug, Event)]
pub struct CellUpdated {
    pos: Position,
    player_id: u64,
}

impl CellUpdated {
    pub fn new(pos: Position, player_id: u64) -> Self {
        Self { pos, player_id }
    }

    pub fn player_id(&self) -> u64 {
        self.player_id
    }

    pub fn pos(&self) -> Position {
        self.pos
    }
}

#[derive(Debug, Event)]
#[allow(dead_code)]
pub struct SuccessfulTurn {
    pos: Position,
    player_id: u64,
}

impl SuccessfulTurn {
    pub fn new(pos: Position, player_id: u64) -> Self {
        Self { pos, player_id }
    }

    #[allow(dead_code)]
    pub fn player_id(&self) -> u64 {
        self.player_id
    }

    #[allow(dead_code)]
    pub fn pos(&self) -> Position {
        self.pos
    }
}

#[derive(Debug, Deref, Event)]
pub struct GameOver(pub FinishedState);
