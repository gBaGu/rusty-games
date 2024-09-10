use bevy::prelude::Component;
use game_server::core::FromProtobuf as _;
use game_server::{core, proto};

use crate::game::{TTTBoard, BOARD_PROTO_SIZE, BOARD_SIZE, PLAYERS_SIZE};

#[derive(Clone, Component, Debug)]
pub struct GameInfo {
    pub id: u64,
    pub players: [u64; PLAYERS_SIZE],
    pub state: core::GameState,
}

impl GameInfo {
    pub fn get_user_id(&self, game_player_id: core::PlayerPosition) -> Option<u64> {
        self.players.get(game_player_id as usize).cloned()
    }
}

impl TryFrom<proto::GameInfo> for GameInfo {
    type Error = core::ProtobufError;

    fn try_from(value: proto::GameInfo) -> Result<Self, Self::Error> {
        let state = value
            .game_state
            .ok_or(core::ProtobufError::MessageDataMissing {
                missing_field: "game_state".to_string(),
            })?;
        let players_len = value.players.len();
        let players =
            value
                .players
                .try_into()
                .map_err(|_| core::ProtobufError::InvalidPlayersLength {
                    expected: PLAYERS_SIZE,
                    found: players_len,
                })?;
        Ok(Self {
            id: value.game_id,
            players,
            state: state.try_into()?,
        })
    }
}

#[derive(Debug)]
pub struct FullGameInfo {
    pub info: GameInfo,
    pub board: TTTBoard,
}

impl TryFrom<proto::GameInfo> for FullGameInfo {
    type Error = core::ProtobufError;

    fn try_from(value: proto::GameInfo) -> Result<Self, Self::Error> {
        let mut full_info = Self {
            info: GameInfo::try_from(value.clone())?,
            board: Default::default(),
        };
        if value.board.is_empty() {
            return Err(core::ProtobufError::MessageDataMissing {
                missing_field: "board".to_string(),
            });
        }
        if value.board.len() != BOARD_PROTO_SIZE {
            return Err(core::ProtobufError::InvalidBoardLength {
                expected: BOARD_PROTO_SIZE,
                found: value.board.len(),
            });
        }
        for (i, row) in value.board.chunks(BOARD_SIZE).enumerate() {
            for (j, val) in row.iter().enumerate() {
                let cell = core::BoardCell::from_protobuf(&val)?;
                full_info.board[(i, j).into()] = cell;
            }
        }
        Ok(full_info)
    }
}
