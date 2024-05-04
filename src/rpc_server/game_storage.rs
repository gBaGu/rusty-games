use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::game::chess::game::Chess;
use crate::game::game::{FromProtobuf, FromProtobufError, Game, GameState};
use crate::game::{error::GameError, player_pool::PlayerId, tic_tac_toe::TicTacToe};
use crate::proto::GameType;

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
    #[error("failed to lock inner mutex: {description}")]
    MutexPoison { description: String },
    #[error("invalid turn data: {source}")]
    TurnDataConversion {
        #[from]
        source: FromProtobufError,
    },
    #[error(transparent)]
    GameError(#[from] GameError),
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
        id: PlayerId,
        players: &[PlayerId],
    ) -> Result<(), GameStorageError> {
        match game_type {
            GameType::TicTacToe => create(&self.tic_tac_toe, id, players),
            GameType::Chess => create(&self.chess, id, players),
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn update_game(
        &self,
        game_type: GameType,
        game_id: PlayerId,
        player_id: PlayerId,
        turn_data: &[u8],
    ) -> Result<GameState, GameStorageError> {
        match game_type {
            GameType::TicTacToe => update(&self.tic_tac_toe, game_id, player_id, turn_data),
            GameType::Chess => update(&self.chess, game_id, player_id, turn_data),
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn delete_game(
        &self,
        game_type: GameType,
        game_id: PlayerId,
    ) -> Result<(), GameStorageError> {
        match game_type {
            GameType::TicTacToe => delete(&self.tic_tac_toe, game_id),
            GameType::Chess => delete(&self.chess, game_id),
            GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }
}

fn create<T: Game>(
    mutex: &Mutex<HashMap<PlayerId, T>>,
    id: PlayerId,
    players: &[PlayerId],
) -> Result<(), GameStorageError> {
    let mut guard = mutex.lock()?;
    match guard.entry(id) {
        Entry::Vacant(e) => {
            let game = T::new(players)?;
            e.insert(game);
        }
        Entry::Occupied(_) => return Err(GameStorageError::DuplicateGame),
    }
    Ok(())
}

fn update<T: Game>(
    mutex: &Mutex<HashMap<PlayerId, T>>,
    id: PlayerId,
    player: PlayerId,
    buf: &[u8],
) -> Result<GameState, GameStorageError> {
    let turn_data = T::TurnData::from_protobuf(buf)?;
    let mut guard = mutex.lock()?;
    let game = guard
        .get_mut(&id)
        .ok_or(GameStorageError::NoSuchGame { id })?;
    Ok(game.update(player, turn_data)?)
}

fn delete<T: Game>(
    mutex: &Mutex<HashMap<PlayerId, T>>,
    id: PlayerId,
) -> Result<(), GameStorageError> {
    let mut guard = mutex.lock()?;
    if let Entry::Occupied(e) = guard.entry(id) {
        if !e.get().is_finished() {
            return Err(GameStorageError::DeleteActiveGameFailed);
        }
        e.remove();
    };
    Ok(())
}
