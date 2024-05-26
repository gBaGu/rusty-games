mod error;

use std::collections::HashMap;

use bevy::asset::Handle;
use bevy::prelude::{Component, Image, Resource};
use game_server::game::encoding::{FromProtobuf, FromProtobufError};
use game_server::game::game::{BoardCell, GameState};
use game_server::game::player_pool::PlayerId;
use game_server::proto;

#[derive(Clone, Copy, Debug, Component)]
pub struct GameCellPosition {
    pub row: u32,
    pub col: u32,
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

#[derive(Resource)]
pub struct CurrentGame {
    id: u64,
    user_id: u64,
    state: GameState,
    images: HashMap<u64, Handle<Image>>,
    board: [[BoardCell<PlayerId>; 3]; 3]
}

impl CurrentGame {
    pub fn new(user_id: u64, game: GameInfo, x_img: Handle<Image>, o_img: Handle<Image>) -> Self {
        Self {
            id: game.id,
            user_id,
            state: game.state,
            images: game.players.into_iter().zip([x_img, o_img]).collect(),
            board: Default::default(),
        }
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn board(&self) -> &[[BoardCell<PlayerId>; 3]] {
        &self.board
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
