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
use crate::game::chess::{self, Chess};
use crate::rpc_server::game_proto::{Coordinates, CoordinatesPair, GameType};

#[derive(thiserror::Error, Debug)]
pub enum GameStorageError {
    #[error("game must be finished before deletion")]
    DeleteActiveGameFailed,
    #[error("this player already has an active game")]
    DuplicateGame,
    #[error("unrecognized game type")]
    InvalidGameType,
    #[error("game with this id doesn't exist: {id}")]
    NoSuchGame { id: PlayerId },
    #[error("turn data has missing field: {missing_field}")]
    TurnDataMissing { missing_field: String },
    #[error("failed to lock inner mutex: {description}")]
    MutexPoison { description: String },
    #[error(transparent)]
    GameError(#[from] GameError),
    #[error(transparent)]
    TurnDataConversion(#[from] TryFromIntError),
    #[error(transparent)]
    ProstDecodeError(#[from] prost::DecodeError),
}

impl<T> From<PoisonError<T>> for GameStorageError {
    fn from(value: PoisonError<T>) -> Self {
        Self::MutexPoison {
            description: value.to_string(),
        }
    }
}

#[derive(Debug, Default)]
pub struct GameStorage {
    tic_tac_toe: Mutex<HashMap<PlayerId, TicTacToe>>,
    chess: Mutex<HashMap<PlayerId, Chess>>,
}

impl GameStorage {
    pub fn create_game(
        &self,
        game_type: GameType,
        player1: PlayerId,
        player2: PlayerId,
    ) -> Result<(), GameStorageError> {
        match game_type {
            GameType::TicTacToe => {
                let mut games_guard = self.tic_tac_toe.lock()?;
                match games_guard.entry(player1) {
                    Entry::Vacant(e) => {
                        let game = TicTacToe::new(player1, player2)?;
                        e.insert(game);
                    }
                    Entry::Occupied(_) => return Err(GameStorageError::DuplicateGame),
                }
            }
            GameType::Chess => {
                let mut games_guard = self.chess.lock()?;
                match games_guard.entry(player1) {
                    Entry::Vacant(e) => {
                        let game = Chess::new(player1, player2)?;
                        e.insert(game);
                    }
                    Entry::Occupied(_) => return Err(GameStorageError::DuplicateGame),
                }
            }
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
        Ok(())
    }

    pub fn update_game(
        &self,
        game_type: GameType,
        game_id: PlayerId,
        player_id: PlayerId,
        turn_data: &[u8],
    ) -> Result<GameState, GameStorageError> {
        match game_type {
            GameType::TicTacToe => {
                let coords = Coordinates::decode(turn_data)?;
                let row: usize = usize::try_from(coords.row)?;
                let col: usize = usize::try_from(coords.col)?;
                let row = tic_tac_toe::FieldRow::try_from(row)?;
                let col = tic_tac_toe::FieldCol::try_from(col)?;
                let mut games_guard = self.tic_tac_toe.lock()?;
                let game = games_guard
                    .get_mut(&game_id)
                    .ok_or(GameStorageError::NoSuchGame { id: game_id })?;
                Ok(game.make_turn(player_id, GridIndex::new(row, col))?)
            }
            GameType::Chess => {
                let turn_data = CoordinatesPair::decode(turn_data)?;
                let first = turn_data.first.ok_or_else(|| GameStorageError::TurnDataMissing { missing_field: "first".to_string() })?;
                let second = turn_data.second.ok_or_else(|| GameStorageError::TurnDataMissing { missing_field: "second".to_string() })?;
                let turn_data= chess::TurnData::new(
                    GridIndex::new(chess::Row(usize::try_from(first.row)?), chess::Col(usize::try_from(first.col)?)),
                    GridIndex::new(chess::Row(usize::try_from(second.row)?), chess::Col(usize::try_from(second.col)?))
                );
                let mut games_guard = self.chess.lock()?;
                let game = games_guard
                    .get_mut(&game_id)
                    .ok_or(GameStorageError::NoSuchGame { id: game_id })?;
                Ok(game.make_turn(player_id, turn_data)?)
            }
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn delete_game(&self, game_type: GameType, game_id: PlayerId) -> Result<(), GameStorageError> {
        match game_type {
            GameType::TicTacToe => {
                let mut games_guard = self.tic_tac_toe.lock()?;
                if let Entry::Occupied(e) = games_guard.entry(game_id) {
                    if !e.get().is_finished() {
                        return Err(GameStorageError::DeleteActiveGameFailed);
                    }
                    e.remove();
                };
            }
            GameType::Chess => {
                let mut games_guard = self.chess.lock()?;
                if let Entry::Occupied(e) = games_guard.entry(game_id) {
                    if !e.get().is_finished() {
                        return Err(GameStorageError::DeleteActiveGameFailed);
                    }
                    e.remove();
                };
            }
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
        Ok(())
    }
}
