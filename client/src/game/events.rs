use bevy::prelude::{Deref, Event};
use game_server::game::game::{FinishedState, GameState};

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

#[derive(Debug, Deref, Event)]
pub struct GameOver(pub FinishedState);
