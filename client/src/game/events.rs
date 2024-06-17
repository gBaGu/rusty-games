use bevy::prelude::{Deref, Event};
use game_server::game::{FinishedState, GameState, PlayerId as GamePlayerId};

use super::{Authority, Position};

#[derive(Debug, Deref, Event)]
pub struct StateUpdated(pub GameState);

#[derive(Debug, Event)]
pub struct CellUpdated {
    pos: Position,
    player_id: GamePlayerId,
}

impl CellUpdated {
    pub fn new(pos: Position, player_id: GamePlayerId) -> Self {
        Self { pos, player_id }
    }

    pub fn player_id(&self) -> GamePlayerId {
        self.player_id
    }

    pub fn pos(&self) -> Position {
        self.pos
    }
}

/// Triggers network game update
#[derive(Debug, Event)]
pub struct NetworkGameTurn {
    pub game_id: u64,
    pub auth: Authority,
    pub pos: Position,
}

/// Triggers local game update
#[derive(Debug, Event)]
pub struct LocalGameTurn {
    pub auth: Authority,
    pub pos: Position,
}

#[derive(Debug, Event)]
pub struct SuccessfulTurn {
    pos: Position,
    player_id: GamePlayerId,
}

impl SuccessfulTurn {
    pub fn new(pos: Position, player_id: GamePlayerId) -> Self {
        Self { pos, player_id }
    }

    #[allow(dead_code)]
    pub fn player_id(&self) -> GamePlayerId {
        self.player_id
    }

    #[allow(dead_code)]
    pub fn pos(&self) -> Position {
        self.pos
    }
}

#[derive(Debug, Deref, Event)]
pub struct GameOver(pub FinishedState);

#[derive(Event)]
pub struct GameExit;
