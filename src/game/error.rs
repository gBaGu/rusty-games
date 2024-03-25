use crate::game::player_pool::PlayerId;
use crate::game::tic_tac_toe::FieldCoordinates;

#[derive(thiserror::Error, Debug)]
pub enum GameError {
    #[error("duplicate player id encountered")]
    DuplicatePlayerId,
    #[error("player not found")]
    PlayerNotFound,
    #[error("invalid row (expected: 1-3, found: {0})")]
    InvalidFieldRow(usize),
    #[error("invalid column (expected: 1-3, found: {0})")]
    InvalidFieldCol(usize),
    #[error("cell ({}, {}) is occupied", .0.get_row(), .0.get_col())]
    CellIsOccupied(FieldCoordinates),
    #[error("can't make turn on a finished game")]
    GameIsFinished,
    #[error("other player's turn (expected: {expected}, found: {found})")]
    NotYourTurn { expected: PlayerId, found: PlayerId },
    #[error("failed to switch players in the pool")]
    PlayerPoolCorrupted,
}
