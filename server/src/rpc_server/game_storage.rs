use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::game::chess::game::Chess;
use crate::game::encoding::{FromProtobuf, FromProtobufError, ToProtobuf};
use crate::game::game::{Game, GameBoard};
use crate::game::player_pool::PlayerQueue;
use crate::game::{error::GameError, player_pool::PlayerId, tic_tac_toe::TicTacToe};
use crate::proto;

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

type GameId = u64;
type GameMap<T> = HashMap<GameId, T>;
type GameStorageResult<T> = Result<T, GameStorageError>;

#[derive(Debug, Default)]
pub struct GameStorage {
    tic_tac_toe: Mutex<GameMap<TicTacToe>>,
    chess: Mutex<GameMap<Chess>>,
}

impl GameStorage {
    pub fn create_game(
        &self,
        game_type: proto::GameType,
        creator: PlayerId,
        players: &[PlayerId],
    ) -> GameStorageResult<GameId> {
        match game_type {
            proto::GameType::TicTacToe => create(&self.tic_tac_toe, creator, players)?,
            proto::GameType::Chess => create(&self.chess, creator, players)?,
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
        Ok(creator)
    }

    pub fn update_game(
        &self,
        game_type: proto::GameType,
        game_id: GameId,
        player_id: PlayerId,
        turn_data: &[u8],
    ) -> GameStorageResult<proto::GameState> {
        match game_type {
            proto::GameType::TicTacToe => update(&self.tic_tac_toe, game_id, player_id, turn_data),
            proto::GameType::Chess => update(&self.chess, game_id, player_id, turn_data),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn delete_game(
        &self,
        game_type: proto::GameType,
        game_id: GameId,
    ) -> GameStorageResult<()> {
        match game_type {
            proto::GameType::TicTacToe => delete(&self.tic_tac_toe, game_id),
            proto::GameType::Chess => delete(&self.chess, game_id),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn get_game(
        &self,
        game_type: proto::GameType,
        player_id: GameId,
    ) -> GameStorageResult<proto::GameInfo> {
        match game_type {
            proto::GameType::TicTacToe => get_game(&self.tic_tac_toe, player_id),
            proto::GameType::Chess => get_game(&self.chess, player_id),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn get_player_games(
        &self,
        game_type: proto::GameType,
        player_id: PlayerId,
    ) -> GameStorageResult<Vec<proto::GameInfo>> {
        match game_type {
            proto::GameType::TicTacToe => get_player_games(&self.tic_tac_toe, player_id),
            proto::GameType::Chess => get_player_games(&self.chess, player_id),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }
}

fn create<T: Game>(
    mutex: &Mutex<GameMap<T>>,
    id: GameId,
    players: &[PlayerId],
) -> GameStorageResult<()> {
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
    mutex: &Mutex<GameMap<T>>,
    id: GameId,
    player: PlayerId,
    buf: &[u8],
) -> GameStorageResult<proto::GameState> {
    let turn_data = T::TurnData::from_protobuf(buf)?;
    let mut guard = mutex.lock()?;
    let game = guard
        .get_mut(&id)
        .ok_or(GameStorageError::NoSuchGame { id })?;
    let game_state = game.update(player, turn_data)?;
    Ok(game_state.into())
}

fn delete<T: Game>(mutex: &Mutex<GameMap<T>>, id: GameId) -> GameStorageResult<()> {
    let mut guard = mutex.lock()?;
    if let Entry::Occupied(e) = guard.entry(id) {
        if !e.get().is_finished() {
            return Err(GameStorageError::DeleteActiveGameFailed);
        }
        e.remove();
    };
    Ok(())
}

fn get_game<T: Game>(mutex: &Mutex<GameMap<T>>, id: GameId) -> GameStorageResult<proto::GameInfo> {
    let guard = mutex.lock()?;
    let game = guard.get(&id).ok_or(GameStorageError::NoSuchGame { id })?;
    let board = game
        .board()
        .get_content()
        .iter()
        .flatten()
        .map(|item| item.to_protobuf())
        .collect();
    Ok(proto::GameInfo {
        game_id: id,
        players: game.get_player_ids(),
        game_state: Some(game.state().into()),
        board,
    })
}

fn get_player_games<T: Game>(
    mutex: &Mutex<GameMap<T>>,
    player_id: PlayerId,
) -> GameStorageResult<Vec<proto::GameInfo>> {
    let guard = mutex.lock()?;
    Ok(guard
        .iter()
        .filter_map(|(id, game)| {
            if game.players().find_by_id(player_id).is_some() {
                return Some(proto::GameInfo {
                    game_id: *id,
                    players: game.get_player_ids(),
                    game_state: Some(game.state().into()),
                    board: vec![],
                });
            }
            None
        })
        .collect())
}
