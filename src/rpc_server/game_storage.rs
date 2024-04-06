use prost::Message;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::num::TryFromIntError;
use std::sync::{Mutex, PoisonError};

use crate::game::{
    error::GameError,
    grid::GridIndex,
    player_pool::PlayerId,
    state::GameState,
    tic_tac_toe::{self, TicTacToe},
};
use crate::rpc_server::game_proto::{Coordinates, GameType};

#[derive(thiserror::Error, Debug)]
pub enum GameStorageError {
    #[error("game must be finished before deletion")]
    DeleteActiveGameFailed,
    #[error("this player already has an active game")]
    DuplicateGame,
    #[error("game with this id doesn't exist: {}", .id)]
    NoSuchGame { id: PlayerId },
    #[error("failed to lock inner mutex: {}", .description)]
    MutexPoisonError { description: String },
    #[error(transparent)]
    GameErrorError(#[from] GameError),
    #[error(transparent)]
    CoordinatesNumericConversion(#[from] TryFromIntError),
    #[error(transparent)]
    ProstError(#[from] prost::DecodeError),
}

impl<T> From<PoisonError<T>> for GameStorageError {
    fn from(value: PoisonError<T>) -> Self {
        Self::MutexPoisonError {
            description: value.to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct GameStorage {
    games: Mutex<HashMap<PlayerId, TicTacToe>>,
}

impl GameStorage {
    pub fn create_game(
        &self,
        game_type: GameType,
        player1: PlayerId,
        player2: PlayerId,
    ) -> Result<(), GameStorageError> {
        let mut games_guard = self.games.lock()?;
        match games_guard.entry(player1) {
            Entry::Vacant(e) => match game_type {
                GameType::TicTacToe => {
                    let game = TicTacToe::new(player1, player2)?;
                    e.insert(game);
                }
            },
            Entry::Occupied(_) => return Err(GameStorageError::DuplicateGame),
        };
        Ok(())
    }

    pub fn update_game(
        &self,
        game_type: GameType,
        game_id: PlayerId,
        player_id: PlayerId,
        turn_data: &[u8],
    ) -> Result<GameState, GameStorageError> {
        let mut games_guard = self.games.lock()?;
        match game_type {
            GameType::TicTacToe => {
                let coords = Coordinates::decode(turn_data)?;
                let row: usize = usize::try_from(coords.row)?;
                let col: usize = usize::try_from(coords.col)?;
                let row = tic_tac_toe::FieldRow::try_from(row)?;
                let col = tic_tac_toe::FieldCol::try_from(col)?;
                let game = games_guard
                    .get_mut(&game_id)
                    .ok_or(GameStorageError::NoSuchGame { id: game_id })?;
                Ok(game.make_turn(player_id, GridIndex::new(row, col))?)
            }
        }
    }

    pub fn delete_game(&self, _: GameType, game_id: PlayerId) -> Result<(), GameStorageError> {
        let mut games_guard = self.games.lock()?;
        if let Entry::Occupied(e) = games_guard.entry(game_id) {
            if !e.get().is_finished() {
                return Err(GameStorageError::DeleteActiveGameFailed);
            }
            e.remove();
        };
        Ok(())
    }
}
