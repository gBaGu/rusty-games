use crate::game::player_pool::PlayerId;

#[derive(thiserror::Error, Debug)]
pub enum GameError {
    #[error("duplicate player id encountered")]
    DuplicatePlayerId,
    #[error("player not found")]
    PlayerNotFound,
    #[error("invalid player number (expected: {expected}, found: {found})")]
    InvalidPlayersNumber { expected: usize, found: usize },
    #[error("cell ({row}, {col}) is empty")]
    CellIsEmpty { row: usize, col: usize },
    #[error("cell ({row}, {col}) is occupied")]
    CellIsOccupied { row: usize, col: usize },
    #[error("can't make turn on a finished game")]
    GameIsFinished,
    #[error("other player's turn (expected: {expected}, found: {found})")]
    NotYourTurn { expected: PlayerId, found: PlayerId },
    #[error("failed to switch players in the pool")]
    PlayerPoolCorrupted,
}
