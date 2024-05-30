use std::collections::HashMap;

use bevy::prelude::*;
use game_server::game::game::{BoardCell, GameState};
use game_server::game::player_pool::PlayerId;

use super::{GAME_REFRESH_INTERVAL_SEC, O_SPRITE_PATH, X_SPRITE_PATH};
use crate::game::GameInfo;

#[derive(Deref, DerefMut, Resource)]
pub struct RefreshGameTimer(pub Timer);

impl Default for RefreshGameTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Resource)]
pub struct CurrentGame {
    id: u64,
    user_id: u64,
    state: GameState,
    images: HashMap<u64, Handle<Image>>,
    board: [[BoardCell<PlayerId>; 3]; 3],
    board_entity: Option<Entity>,
}

impl CurrentGame {
    pub fn new(user_id: u64, game: GameInfo, x_img: Handle<Image>, o_img: Handle<Image>) -> Self {
        Self {
            id: game.id,
            user_id,
            state: game.state,
            images: game.players.into_iter().zip([x_img, o_img]).collect(),
            board: Default::default(),
            board_entity: None,
        }
    }

    pub fn new_from_asset_server(user_id: u64, game: GameInfo, asset_server: &AssetServer) -> Self {
        let x_img = asset_server.load(X_SPRITE_PATH);
        let o_img = asset_server.load(O_SPRITE_PATH);
        Self::new(user_id, game, x_img, o_img)
    }

    pub fn id(&self) -> u64 {
        self.id
    }

    pub fn user_id(&self) -> u64 {
        self.user_id
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn board(&self) -> &[[BoardCell<PlayerId>; 3]] {
        &self.board
    }

    pub fn board_entity(&self) -> &Option<Entity> {
        &self.board_entity
    }

    pub fn get_enemy(&self) -> Option<u64> {
        self.images.keys().find(|&&k| k != self.user_id).cloned()
    }

    pub fn get_next_player(&self) -> Option<u64> {
        if let GameState::Turn(id) = self.state {
            return Some(id);
        }
        None
    }

    #[allow(dead_code)]
    pub fn get_next_player_image(&self) -> Option<&Handle<Image>> {
        if let GameState::Turn(id) = self.state {
            return self.get_player_image(&id);
        }
        None
    }

    pub fn get_player_image(&self, id: &u64) -> Option<&Handle<Image>> {
        self.images.get(id)
    }

    pub fn set_board(&mut self, board: Entity) {
        self.board_entity = Some(board);
    }

    pub fn set_cell(&mut self, pos: (usize, usize), player_id: PlayerId) {
        self.board[pos.0][pos.1] = player_id.into()
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }
}
