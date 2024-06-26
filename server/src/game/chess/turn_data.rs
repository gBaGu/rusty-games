use prost::Message;

use crate::game::encoding::{FromProtobuf, FromProtobufError};
use crate::game::grid::GridIndex;
use crate::proto::PositionPair;

#[derive(Clone, Copy, Debug)]
pub struct TurnData {
    pub from: GridIndex,
    pub to: GridIndex,
}

impl TurnData {
    pub fn new(from: GridIndex, to: GridIndex) -> Self {
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
            GridIndex::new(
                usize::try_from(first.row)?,
                usize::try_from(first.col)?,
            ),
            GridIndex::new(
                usize::try_from(second.row)?,
                usize::try_from(second.col)?,
            ),
        );
        Ok(turn_data)
    }
}
