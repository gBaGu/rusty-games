mod error;

use std::collections::HashMap;

use bevy::asset::Handle;
use bevy::prelude::{Component, Image, Resource};
use game_server::game::game::GameState;
use game_server::game::encoding::FromProtobufError;
use game_server::proto;
use prost::Message;

#[derive(Clone, Copy, Debug, Component)]
pub struct GameCellPosition {
    row: u32,
    col: u32,
}

impl GameCellPosition {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    pub fn row(&self) -> u32 {
        self.row
    }

    pub fn col(&self) -> u32 {
        self.col
    }
}

impl From<GameCellPosition> for proto::Position {
    fn from(value: GameCellPosition) -> Self {
        Self {
            row: value.row,
            col: value.col,
        }
    }
}

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

pub struct FullGameInto {
    pub info: GameInfo,
    pub board: Vec<Vec<u64>>,
}

impl TryFrom<proto::GameInfo> for FullGameInto {
    type Error = FromProtobufError;

    fn try_from(value: proto::GameInfo) -> Result<Self, Self::Error> {
        let mut full_info = Self {
            info: GameInfo::try_from(value.clone())?,
            board: Vec::<Vec<u64>>::with_capacity(3),
        };
        if value.board.is_empty() {
            return Err(FromProtobufError::MessageDataMissing {
                missing_field: "board".to_string(),
            });
        }
        for row in value.board.chunks(3) {
            let row: Vec<_> = row
                .iter()
                .map(|val| u64::decode(val.as_slice()))
                .collect::<Result<_, _>>()?;
            if row.len() != 3 {
                return Err(FromProtobufError::InvalidProtobufMessage {
                    reason: format!("invalid board length: expected=9, found={}", value.board.len())
                });
            }
            full_info.board.push(row);
        }
        if full_info.board.len() != 3 {
            return Err(FromProtobufError::InvalidProtobufMessage {
                reason: format!("invalid board length: expected=9, found={}", value.board.len())
            });
        }

        Ok(full_info)
    }
}

#[derive(Resource)]
pub struct CurrentGame {
    id: u64,
    user_id: u64,
    state: GameState,
    images: HashMap<u64, Handle<Image>>,
}

impl CurrentGame {
    pub fn new(user_id: u64, game: GameInfo, x_img: Handle<Image>, o_img: Handle<Image>) -> Self {
        Self {
            id: game.id,
            user_id,
            state: game.state,
            images: game.players.into_iter().zip([x_img, o_img]).collect(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn get_next_player(&self) -> Option<u64> {
        if let GameState::Turn(id) = self.state {
            return Some(id);
        }
        None
    }

    pub fn get_next_player_image(&self) -> Option<&Handle<Image>> {
        if let GameState::Turn(id) = self.state {
            return self.get_player_image(&id);
        }
        None
    }

    pub fn get_player_image(&self, id: &u64) -> Option<&Handle<Image>> {
        self.images.get(id)
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }
}
