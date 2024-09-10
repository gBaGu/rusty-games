use bevy::prelude::*;
use game_server::core;

/// Event emitted when board tile is pressed.
/// Contains entity of a board whose tile was pressed
/// and a [`GridIndex`] of a button.
#[derive(Debug, Event)]
pub struct TilePressed {
    board: Entity,
    pos: core::GridIndex,
}

impl TilePressed {
    pub fn new(board: Entity, pos: core::GridIndex) -> Self {
        Self { board, pos }
    }

    pub fn board(&self) -> Entity {
        self.board
    }

    pub fn pos(&self) -> core::GridIndex {
        self.pos
    }
}
