use crate::game::player_pool::PlayerId;

#[derive(thiserror::Error, Debug, PartialEq)]
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
    #[error("failed to make move: {reason}")]
    InvalidMove { reason: String },
    #[error("player {found} is unable to make this move, player {expected} is expected")]
    UnauthorizedMove { expected: PlayerId, found: PlayerId },
    #[error("failed to switch players in the pool")]
    PlayerPoolCorrupted,
}

impl GameError {
    pub fn cell_is_empty(row: usize, col: usize) -> Self {
        Self::CellIsEmpty { row, col }
    }

    pub fn cell_is_occupied(row: usize, col: usize) -> Self {
        Self::CellIsOccupied { row, col }
    }

    pub fn invalid_move(reason: String) -> Self {
        Self::InvalidMove { reason }
    }

    pub fn invalid_players_number(expected: usize, found: usize) -> Self {
        Self::InvalidPlayersNumber { expected, found }
    }

    pub fn not_your_turn(expected: PlayerId, found: PlayerId) -> Self {
        Self::NotYourTurn { expected, found }
    }

    pub fn unauthorized_move(expected: PlayerId, found: PlayerId) -> Self {
        Self::UnauthorizedMove { expected, found }
    }
}
