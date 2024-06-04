use prost::Message;
use std::num::TryFromIntError;

use crate::game::BoardCell;
use crate::proto::Maybe;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FromProtobufError {
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
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError>;
}

pub trait ToProtobuf {
    fn to_protobuf(self) -> Vec<u8>;
}

impl<T: Default + prost::Message> FromProtobuf for T {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        Ok(T::decode(buf)?)
    }
}

impl<T: FromProtobuf> FromProtobuf for BoardCell<T> {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        let maybe = Maybe::decode(buf)?;
        match maybe.item {
            Some(bytes) => Ok(T::from_protobuf(&bytes)?.into()),
            None => Ok(Self::default()),
        }
    }
}

impl<T: prost::Message> ToProtobuf for T {
    fn to_protobuf(self) -> Vec<u8> {
        self.encode_to_vec()
    }
}

impl<T: ToProtobuf> ToProtobuf for BoardCell<T> {
    fn to_protobuf(self) -> Vec<u8> {
        let mut maybe = Maybe::default();
        if let BoardCell(Some(val)) = self {
            maybe.item = Some(val.to_protobuf());
        }
        maybe.encode_to_vec()
    }
}
