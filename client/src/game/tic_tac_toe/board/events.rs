use bevy::prelude::*;
use game_server::game::grid::GridIndex;

/// Event emitted when board tile is pressed.
/// Contains entity of a board whose tile was pressed
/// and a [`GridIndex`] of a button.
#[derive(Debug, Event)]
pub struct TilePressed {
    board: Entity,
    pos: GridIndex,
}

impl TilePressed {
    pub fn new(board: Entity, pos: GridIndex) -> Self {
        Self { board, pos }
    }

    pub fn board(&self) -> Entity {
        self.board
    }

    pub fn pos(&self) -> GridIndex {
        self.pos
    }
}
