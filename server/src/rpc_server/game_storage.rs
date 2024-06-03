use smallvec::SmallVec;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::{Mutex, PoisonError};

use crate::game::chess::game::Chess;
use crate::game::encoding::{FromProtobuf, FromProtobufError, ToProtobuf};
use crate::game::{error::GameError, tic_tac_toe::TicTacToe};
use crate::game::{Game, GameBoard};
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
    NoSuchGame { id: GameId },
    #[error("player {player} does not belong to game {game}")]
    ForeignGame { game: GameId, player: UserId },
    #[error("internal error: {reason}")]
    Internal { reason: String },
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

pub type GameId = u64;

type UserId = u64;
type GameMap<T> = HashMap<GameId, (T, SmallVec<[UserId; 8]>)>;
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
        creator: UserId,
        players: &[UserId],
    ) -> GameStorageResult<proto::GameInfo> {
        match game_type {
            proto::GameType::TicTacToe => create(&self.tic_tac_toe, creator, players),
            proto::GameType::Chess => create(&self.chess, creator, players),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn update_game(
        &self,
        game_type: proto::GameType,
        game: GameId,
        player: UserId,
        turn_data: &[u8],
    ) -> GameStorageResult<proto::GameState> {
        match game_type {
            proto::GameType::TicTacToe => update(&self.tic_tac_toe, game, player, turn_data),
            proto::GameType::Chess => update(&self.chess, game, player, turn_data),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn delete_game(&self, game_type: proto::GameType, game: GameId) -> GameStorageResult<()> {
        match game_type {
            proto::GameType::TicTacToe => delete(&self.tic_tac_toe, game),
            proto::GameType::Chess => delete(&self.chess, game),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn get_game(
        &self,
        game_type: proto::GameType,
        player: GameId,
    ) -> GameStorageResult<proto::GameInfo> {
        match game_type {
            proto::GameType::TicTacToe => get_game(&self.tic_tac_toe, player),
            proto::GameType::Chess => get_game(&self.chess, player),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }

    pub fn get_player_games(
        &self,
        game_type: proto::GameType,
        player: UserId,
    ) -> GameStorageResult<Vec<proto::GameInfo>> {
        match game_type {
            proto::GameType::TicTacToe => get_player_games(&self.tic_tac_toe, player),
            proto::GameType::Chess => get_player_games(&self.chess, player),
            proto::GameType::Unspecified => return Err(GameStorageError::InvalidGameType),
        }
    }
}

fn create<T: Game>(
    mutex: &Mutex<GameMap<T>>,
    id: GameId,
    players: &[UserId],
) -> GameStorageResult<proto::GameInfo> {
    let mut guard = mutex.lock()?;
    return match guard.entry(id) {
        Entry::Vacant(e) => {
            let game = T::new();
            let info = proto::GameInfo {
                game_id: id,
                players: players.to_vec(),
                game_state: Some(game.state().into()),
                board: vec![],
            };
            e.insert((game, SmallVec::from_slice(players)));
            Ok(info)
        }
        Entry::Occupied(_) => Err(GameStorageError::DuplicateGame),
    };
}

fn update<T: Game>(
    mutex: &Mutex<GameMap<T>>,
    id: GameId,
    player: UserId,
    buf: &[u8],
) -> GameStorageResult<proto::GameState> {
    let turn_data = T::TurnData::from_protobuf(buf)?;
    let mut guard = mutex.lock()?;
    let (game, players) = guard
        .get_mut(&id)
        .ok_or(GameStorageError::NoSuchGame { id })?;
    let player = players
        .iter()
        .position(|&id| id == player)
        .ok_or(GameStorageError::ForeignGame { game: id, player })?
        .try_into()
        .map_err(|_| GameStorageError::Internal {
            reason: "internal player index is out of bounds".into(),
        })?;
    let game_state = game.update(player, turn_data)?;
    Ok(game_state.into())
}

fn delete<T: Game>(mutex: &Mutex<GameMap<T>>, id: GameId) -> GameStorageResult<()> {
    let mut guard = mutex.lock()?;
    if let Entry::Occupied(e) = guard.entry(id) {
        if !e.get().0.is_finished() {
            return Err(GameStorageError::DeleteActiveGameFailed);
        }
        e.remove();
    };
    Ok(())
}

fn get_game<T: Game>(mutex: &Mutex<GameMap<T>>, id: GameId) -> GameStorageResult<proto::GameInfo> {
    let guard = mutex.lock()?;
    let (game, players) = guard.get(&id).ok_or(GameStorageError::NoSuchGame { id })?;
    let board = game
        .board()
        .get_content()
        .into_iter()
        .flatten()
        .map(|item| item.to_protobuf())
        .collect();
    Ok(proto::GameInfo {
        game_id: id,
        players: players.to_vec(),
        game_state: Some(game.state().into()),
        board,
    })
}

fn get_player_games<T: Game>(
    mutex: &Mutex<GameMap<T>>,
    player: UserId,
) -> GameStorageResult<Vec<proto::GameInfo>> {
    let guard = mutex.lock()?;
    Ok(guard
        .iter()
        .filter_map(|(id, (game, players))| {
            if players.contains(&player) {
                return Some(proto::GameInfo {
                    game_id: *id,
                    players: players.to_vec(),
                    game_state: Some(game.state().into()),
                    board: vec![],
                });
            }
            None
        })
        .collect())
}
