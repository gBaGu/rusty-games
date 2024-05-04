use std::num::TryFromIntError;
use crate::game::error::GameError;
use crate::game::player_pool::PlayerId;
use crate::game::state::GameState;

pub type GameResult<T> = Result<T, GameError>;

#[derive(thiserror::Error, Debug)]
pub enum FromProtobufError {
    #[error("invalid row (expected: 1-{max_expected}, found: {found})")]
    InvalidGridRow { max_expected: usize, found: usize },
    #[error("invalid column (expected: 1-{max_expected}, found: {found})")]
    InvalidGridCol { max_expected: usize, found: usize },
    #[error("turn data has missing field: {missing_field}")]
    TurnDataMissing { missing_field: String },
    #[error(transparent)]
    TurnDataConversion(#[from] TryFromIntError),
    #[error(transparent)]
    ProstDecodeError(#[from] prost::DecodeError),
}

pub trait FromProtobuf: Sized {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError>;
}

pub trait Game: Sized {
    type TurnData: FromProtobuf;

    fn new(players: &[PlayerId]) -> GameResult<Self>;
    fn state(&self) -> GameState;
    fn update(&mut self, player: PlayerId, data: Self::TurnData) -> GameResult<GameState>;

    fn is_finished(&self) -> bool {
        matches!(self.state(), GameState::Finished(_))
    }
}
