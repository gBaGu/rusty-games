use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::ops::Deref;
use std::sync::{Arc, Mutex};

use super::error::RpcError;
use super::lobby::{Connection, Lobby};
use super::rpc::{GameId, RpcInnerResult, UserId};
use crate::core::{Game, GameBoard, GameState, ProtobufResult, ToProtobuf};
use crate::proto;

type GameMap<T> = HashMap<GameId, Lobby<T>>;

pub struct GameStorage<T>(Arc<Mutex<GameMap<T>>>);

impl<T> Clone for GameStorage<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T> Default for GameStorage<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T> Deref for GameStorage<T> {
    type Target = Arc<Mutex<GameMap<T>>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> GameStorage<T> {
    /// Remove connection and wait for connection task.
    pub async fn disconnect(&self, game: GameId, user: UserId) -> RpcInnerResult<()> {
        let Some(mut conn) = self.remove_connection(game, user)? else {
            return Ok(());
        };
        if let Err(err) = conn.wait().await {
            return Err(RpcError::ConnectionJoinError(err.to_string()));
        }
        Ok(())
    }

    /// Remove `user` connection from the lobby indicated by `game`, return if removed.
    pub fn remove_connection(&self, game: GameId, user: UserId) -> RpcInnerResult<Option<Connection>> {
        let mut guard = self.lock()?;
        Ok(guard
            .get_mut(&game)
            .and_then(|lobby| lobby.remove_connection(user)))
    }

    /// Notify `user` of an error.
    pub fn notify_err(&self, game: GameId, user: UserId, err: RpcError) -> RpcInnerResult<()> {
        let mut guard = self.lock()?;
        if let Some(lobby) = guard.get_mut(&game) {
            lobby.notify_err(user, err)?;
        }
        Ok(())
    }
}

impl<T: Game> GameStorage<T> {
    // TODO: replace proto::GameInfo with own GameInfo type (move it from client)
    pub fn create(&self, id: GameId, players: &[UserId]) -> RpcInnerResult<proto::GameInfo> {
        let mut guard = self.lock()?;
        return match guard.entry(id) {
            Entry::Vacant(e) => {
                let lobby = Lobby::<T>::new(players);
                let info = proto::GameInfo {
                    game_id: id,
                    players: players.to_vec(),
                    game_state: Some(lobby.game().state().into()),
                    board: vec![],
                };
                e.insert(lobby);
                Ok(info)
            }
            Entry::Occupied(_) => Err(RpcError::DuplicateGame),
        };
    }

    pub fn update(&self, id: GameId, player: UserId, data: &[u8]) -> RpcInnerResult<GameState> {
        let mut guard = self.lock()?;
        let lobby = guard.get_mut(&id).ok_or(RpcError::NoSuchGame { id })?;
        lobby.update(player, data)
    }

    pub fn delete(&self, id: GameId) -> RpcInnerResult<()> {
        let mut guard = self.lock()?;
        if let Entry::Occupied(e) = guard.entry(id) {
            if !e.get().game().is_finished() {
                return Err(RpcError::DeleteActiveGameFailed);
            }
            e.remove();
        };
        Ok(())
    }

    pub fn get(&self, id: GameId) -> RpcInnerResult<proto::GameInfo> {
        let guard = self.lock()?;
        let lobby = guard.get(&id).ok_or(RpcError::NoSuchGame { id })?;
        let board = lobby
            .game()
            .board()
            .get_content()
            .into_iter()
            .flatten()
            .map(|item| item.to_protobuf())
            .collect::<ProtobufResult<Vec<_>>>()?;
        Ok(proto::GameInfo {
            game_id: id,
            players: lobby.players().to_vec(),
            game_state: Some(lobby.game().state().into()),
            board,
        })
    }

    pub fn get_player_games(&self, player: UserId) -> RpcInnerResult<Vec<proto::GameInfo>> {
        let guard = self.lock()?;
        Ok(guard
            .iter()
            .filter_map(|(id, lobby)| {
                if lobby.players().contains(&player) {
                    return Some(proto::GameInfo {
                        game_id: *id,
                        players: lobby.players().to_vec(),
                        game_state: Some(lobby.game().state().into()),
                        board: vec![],
                    });
                }
                None
            })
            .collect())
    }
}
