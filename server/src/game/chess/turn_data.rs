use prost::Message;

use crate::game::chess::index::{Col, Index, Row};
use crate::game::game::{FromProtobuf, FromProtobufError};
use crate::proto::PositionPair;

#[derive(Clone, Copy, Debug)]
pub struct TurnData {
    pub from: Index,
    pub to: Index,
}

impl TurnData {
    pub fn new(from: Index, to: Index) -> Self {
        Self { from, to }
    }
}

impl FromProtobuf for TurnData {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError> {
        let pos = PositionPair::decode(buf)?;
        let first = pos
            .first
            .ok_or_else(|| FromProtobufError::MessageDataMissing {
                missing_field: "first".to_string(),
            })?;
        let second = pos
            .second
            .ok_or_else(|| FromProtobufError::MessageDataMissing {
                missing_field: "second".to_string(),
            })?;
        let turn_data = TurnData::new(
            Index::new(
                Row(usize::try_from(first.row)?),
                Col(usize::try_from(first.col)?),
            ),
            Index::new(
                Row(usize::try_from(second.row)?),
                Col(usize::try_from(second.col)?),
            ),
        );
        Ok(turn_data)
    }
}
