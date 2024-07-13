use bevy::prelude::{Deref, Event};
use game_server::game::{GameState, PlayerId as PlayerPosition};

use super::{Authority, Position};

#[derive(Debug, Deref, Event)]
pub struct StateUpdated(pub GameState);

#[derive(Debug, Event)]
pub struct CellUpdated {
    pos: Position,
    player_position: PlayerPosition,
}

impl CellUpdated {
    pub fn new(pos: Position, player_position: PlayerPosition) -> Self {
        Self { pos, player_position }
    }

    pub fn player_position(&self) -> PlayerPosition {
        self.player_position
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

/// General game event
/// Indicates that player finished its turn
#[derive(Debug, Event)]
pub struct SuccessfulTurn {
    player: PlayerPosition,
}

impl SuccessfulTurn {
    pub fn new(player: PlayerPosition) -> Self {
        Self { player }
    }

    #[allow(dead_code)]
    pub fn player(&self) -> PlayerPosition {
        self.player
    }
}

#[derive(Debug, Deref, Event)]
pub struct PlayerWon(pub Authority);

#[derive(Debug, Event)]
pub struct GameOver {
    winner: Option<Authority>,
}

impl GameOver {
    pub fn new(winner: Option<Authority>) -> Self {
        Self { winner }
    }

    pub fn winner(&self) -> Option<Authority> {
        self.winner
    }
}
