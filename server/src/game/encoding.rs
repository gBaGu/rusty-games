use std::num::TryFromIntError;
use prost::Message;
use crate::game::game::BoardCell;

use crate::proto::Maybe;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FromProtobufError {
    #[error("invalid row (expected: 1-{max_expected}, found: {found})")]
    InvalidGridRow { max_expected: usize, found: usize },
    #[error("invalid column (expected: 1-{max_expected}, found: {found})")]
    InvalidGridCol { max_expected: usize, found: usize },
    #[error("protobuf message is invalid: {reason}")]
    InvalidProtobufMessage { reason: String },
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
    fn to_protobuf(&self) -> Vec<u8>;
}

impl<T: prost::Message> ToProtobuf for T {
    fn to_protobuf(&self) -> Vec<u8> {
        self.encode_to_vec()
    }
}

impl<T: ToProtobuf> ToProtobuf for BoardCell<T> {
    fn to_protobuf(&self) -> Vec<u8> {
        let item = self.as_ref().and_then(|val| {
            Some(val.to_protobuf())
        });
        Maybe { item }.encode_to_vec()
    }
}
