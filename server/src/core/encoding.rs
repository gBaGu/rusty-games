use std::num::TryFromIntError;

use prost::Message;

use crate::core::{BoardCell, GridIndex};
use crate::proto::{Maybe, Position};

pub type ProtobufResult<T> = Result<T, ProtobufError>;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum ProtobufError {
    #[error("invalid row (expected: 1-{max_expected}, found: {found})")]
    InvalidGridRow { max_expected: usize, found: usize },
    #[error("invalid column (expected: 1-{max_expected}, found: {found})")]
    InvalidGridCol { max_expected: usize, found: usize },
    #[error("invalid players list length: expected={expected}, found={found}")]
    InvalidPlayersLength { expected: usize, found: usize },
    #[error("invalid board length: expected={expected}, found={found}")]
    InvalidBoardLength { expected: usize, found: usize },
    #[error("game state is invalid")]
    InvalidGameState,
    #[error("message data has missing field: {missing_field}")]
    MessageDataMissing { missing_field: String },
    #[error(transparent)]
    TurnDataConversion(#[from] TryFromIntError),
    #[error(transparent)]
    ProstDecodeError(#[from] prost::DecodeError),
}

pub trait FromProtobuf: Sized {
    fn from_protobuf(buf: &[u8]) -> ProtobufResult<Self>;
}

pub trait ToProtobuf {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>>;
}

impl<T: Default + prost::Message> FromProtobuf for T {
    fn from_protobuf(buf: &[u8]) -> ProtobufResult<Self> {
        Ok(T::decode(buf)?)
    }
}

impl<T: FromProtobuf> FromProtobuf for BoardCell<T> {
    fn from_protobuf(buf: &[u8]) -> ProtobufResult<Self> {
        let maybe = Maybe::decode(buf)?;
        match maybe.item {
            Some(bytes) => Ok(T::from_protobuf(&bytes)?.into()),
            None => Ok(Self::default()),
        }
    }
}

impl FromProtobuf for GridIndex {
    fn from_protobuf(buf: &[u8]) -> Result<Self, ProtobufError> {
        let pos = Position::decode(buf)?;
        let row: usize = usize::try_from(pos.row)?;
        let col: usize = usize::try_from(pos.col)?;
        Ok(Self::new(row, col))
    }
}

impl<T: prost::Message> ToProtobuf for T {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>> {
        Ok(self.encode_to_vec())
    }
}

impl<T: ToProtobuf> ToProtobuf for BoardCell<T> {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>> {
        let mut maybe = Maybe::default();
        if let BoardCell(Some(val)) = self {
            maybe.item = Some(val.to_protobuf()?);
        }
        Ok(maybe.encode_to_vec())
    }
}

impl ToProtobuf for GridIndex {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>> {
        Position::try_from(self)?.to_protobuf()
    }
}
