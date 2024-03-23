use crate::game::tic_tac_toe::{FieldCoordinates, GameState, PlayerId, TicTacToe, TicTacToeError};
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::sync::Mutex;

#[derive(thiserror::Error, Debug)]
pub enum GameStorageError {
    #[error("game must be finished before deletion")]
    DeleteActiveGameFailed,
    #[error("this player already has an active game")]
    DuplicateGame,
    #[error("game with this id doesn't exist: {}", .id)]
    NoSuchGame { id: PlayerId },
    #[error("failed to lock inner mutex")]
    MutexLockFailed,
    #[error(transparent)]
    TicTacToeError(#[from] TicTacToeError),
}

#[derive(Debug, Default)]
pub struct GameStorage {
    games: Mutex<HashMap<PlayerId, TicTacToe>>,
}

impl GameStorage {
    pub fn create_game(
        &self,
        player1: PlayerId,
        player2: PlayerId,
    ) -> Result<(), GameStorageError> {
        let mut games_guard = self.games.lock().map_err(|_| GameStorageError::MutexLockFailed)?;
        match games_guard.entry(player1) {
            Entry::Vacant(e) => {
                let game = TicTacToe::new(player1, player2)?;
                e.insert(game);
            }
            Entry::Occupied(_) => return Err(GameStorageError::DuplicateGame),
        };
        Ok(())
    }

    pub fn update_game(
        &self,
        game_id: PlayerId,
        player_id: PlayerId,
        coords: FieldCoordinates,
    ) -> Result<GameState, GameStorageError> {
        let mut games_guard = self.games.lock().map_err(|_| GameStorageError::MutexLockFailed)?;
        let game = games_guard
            .get_mut(&game_id)
            .ok_or(GameStorageError::NoSuchGame { id: game_id })?;
        Ok(game.make_turn(player_id, coords)?)
    }

    pub fn delete_game(&self, game_id: PlayerId) -> Result<(), GameStorageError> {
        let mut games_guard = self.games.lock().map_err(|_| GameStorageError::MutexLockFailed)?;
        if let Entry::Occupied(e) = games_guard.entry(game_id) {
            if !e.get().is_finished() {
                return Err(GameStorageError::DeleteActiveGameFailed);
            }
            e.remove();
        };
        Ok(())
    }
}
