use prost::Message;

use crate::core::{FromProtobuf, GridIndex, ProtobufError, ProtobufResult, ToProtobuf};
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
    fn from_protobuf(buf: &[u8]) -> Result<Self, ProtobufError> {
        let pos = PositionPair::decode(buf)?;
        let first = pos.first.ok_or_else(|| ProtobufError::MessageDataMissing {
            missing_field: "first".to_string(),
        })?;
        let second = pos
            .second
            .ok_or_else(|| ProtobufError::MessageDataMissing {
                missing_field: "second".to_string(),
            })?;
        let turn_data = TurnData::new(
            GridIndex::new(usize::try_from(first.row)?, usize::try_from(first.col)?),
            GridIndex::new(usize::try_from(second.row)?, usize::try_from(second.col)?),
        );
        Ok(turn_data)
    }
}

impl ToProtobuf for TurnData {
    fn to_protobuf(self) -> ProtobufResult<Vec<u8>> {
        PositionPair::try_from(self)?.to_protobuf()
    }
}
