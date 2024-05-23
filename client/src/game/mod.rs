mod error;

use std::collections::HashMap;

use bevy::asset::Handle;
use bevy::prelude::{Component, Image, Resource};
use game_server::game::game::{FinishedState, GameState};

use crate::grpc::proto;

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
    type Error = error::GameError;

    fn try_from(value: proto::GameInfo) -> Result<Self, error::GameError> {
        let state = value
            .game_state
            .ok_or(error::GameError::InvalidProtobufMessage {
                reason: "missing game_state".to_string(),
            })?;
        let state = match (state.next_player_id, state.winner) {
            (Some(next), None) => GameState::Turn(next),
            (None, Some(winner)) => GameState::Finished(FinishedState::Win(winner)),
            (None, None) => GameState::Finished(FinishedState::Draw),
            _ => {
                return Err(error::GameError::InvalidProtobufMessage {
                    reason: "invalid game_state".to_string(),
                })
            }
        };
        Ok(Self {
            id: value.game_id,
            players: value.players,
            state,
        })
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
