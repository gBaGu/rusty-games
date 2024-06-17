use bevy::prelude::*;

use crate::game::Position;

/// Event emitted when board tile is pressed.
/// Contains entity of a board whose tile was pressed
/// and a [`Position`] of a button.
#[derive(Debug, Event)]
pub struct TilePressed {
    #[allow(dead_code)]
    board: Entity,
    pos: Position,
}

impl TilePressed {
    pub fn new(board: Entity, pos: Position) -> Self {
        Self { board, pos }
    }

    pub fn pos(&self) -> Position {
        self.pos
    }
}

/// Event used by this plugin to update image of a tile.
/// Contains entity of a board whose tile needs to be updated,
/// button [`Position`] and an image.
#[derive(Clone, Debug, Event)]
pub struct TileFilled {
    board: Entity,
    pos: Position,
    image: Handle<Image>,
}

impl TileFilled {
    pub fn new(board: Entity, pos: Position, image: Handle<Image>) -> Self {
        Self { board, pos, image }
    }

    pub fn board(&self) -> Entity {
        self.board
    }

    pub fn pos(&self) -> Position {
        self.pos
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }
}
