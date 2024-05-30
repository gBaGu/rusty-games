use bevy::prelude::Component;
use game_server::game::encoding::{FromProtobuf, FromProtobufError};
use game_server::game::game::{BoardCell, GameState};
use game_server::game::player_pool::PlayerId;
use game_server::proto;

#[derive(Clone, Component, Debug)]
pub struct GameInfo {
    pub id: u64,
    pub players: Vec<u64>,
    pub state: GameState,
}

impl TryFrom<proto::GameInfo> for GameInfo {
    type Error = FromProtobufError;

    fn try_from(value: proto::GameInfo) -> Result<Self, Self::Error> {
        let state = value
            .game_state
            .ok_or(FromProtobufError::MessageDataMissing {
                missing_field: "game_state".to_string(),
            })?;
        Ok(Self {
            id: value.game_id,
            players: value.players,
            state: state.try_into()?,
        })
    }
}

#[derive(Debug)]
pub struct FullGameInfo {
    pub info: GameInfo,
    pub board: [[BoardCell<PlayerId>; 3]; 3],
}

impl TryFrom<proto::GameInfo> for FullGameInfo {
    type Error = FromProtobufError;

    fn try_from(value: proto::GameInfo) -> Result<Self, Self::Error> {
        let mut full_info = Self {
            info: GameInfo::try_from(value.clone())?,
            board: Default::default(),
        };
        if value.board.is_empty() {
            return Err(FromProtobufError::MessageDataMissing {
                missing_field: "board".to_string(),
            });
        }
        if value.board.len() != 9 {
            return Err(FromProtobufError::InvalidProtobufMessage {
                reason: format!(
                    "invalid board length: expected=9, found={}",
                    value.board.len()
                ),
            });
        }
        for (i, row) in value.board.chunks(3).enumerate() {
            for (j, val) in row.iter().enumerate() {
                let cell = BoardCell::from_protobuf(&val)?;
                full_info.board[i][j] = cell;
            }
        }
        Ok(full_info)
    }
}
