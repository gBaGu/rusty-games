use bevy::prelude::*;
use game_server::game::grid::GridIndex;

/// Component that stores a position inside the board.
#[derive(Clone, Copy, Debug, PartialEq, Component, Deref, DerefMut)]
pub struct Position(GridIndex);

impl Position {
    pub fn new(row: usize, col: usize) -> Self {
        Self(GridIndex::new(row, col))
    }
}

impl From<GridIndex> for Position {
    fn from(value: GridIndex) -> Self {
        Self(value)
    }
}

impl From<Position> for GridIndex {
    fn from(value: Position) -> Self {
        value.0
    }
}
