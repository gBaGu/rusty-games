use bevy::prelude::Component;

/// Component that stores a position inside the board.
#[derive(Clone, Copy, Debug, PartialEq, Component)]
pub struct Position {
    row: u32,
    col: u32,
}

impl Position {
    /// Creates [`Position`] from row and col
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    /// Getter for `self.row`
    pub fn row(&self) -> u32 {
        self.row
    }

    /// Getter for `self.col`
    pub fn col(&self) -> u32 {
        self.col
    }
}
