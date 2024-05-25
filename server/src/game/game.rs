use std::ops::{Deref, DerefMut};
use crate::game::encoding::{FromProtobuf, FromProtobufError, ToProtobuf};
use crate::game::error::GameError;
use crate::game::grid::{Grid, WithLength};
use crate::game::player_pool::{PlayerId, PlayerQueue, WithPlayerId};

pub type GameResult<T> = Result<T, GameError>;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoardCell<T>(pub Option<T>);

impl<T> Default for BoardCell<T> {
    fn default() -> Self {
        Self(Option::default())
    }
}

impl<T> From<T> for BoardCell<T> {
    fn from(value: T) -> Self {
        Self(Option::from(value))
    }
}

impl<T> Deref for BoardCell<T> {
    type Target = Option<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> DerefMut for BoardCell<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FinishedState {
    Win(PlayerId),
    Draw,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameState {
    Turn(PlayerId),
    Finished(FinishedState),
}

impl TryFrom<crate::proto::GameState> for GameState {
    type Error = FromProtobufError;

    fn try_from(value: crate::proto::GameState) -> Result<Self, Self::Error> {
        let state = match (value.next_player_id, value.winner) {
            (Some(next), None) => GameState::Turn(next),
            (None, Some(winner)) => GameState::Finished(FinishedState::Win(winner)),
            (None, None) => GameState::Finished(FinishedState::Draw),
            _ => {
                return Err(FromProtobufError::InvalidProtobufMessage {
                    reason: "invalid game_state".to_string(),
                })
            }
        };
        Ok(state)
    }
}

pub trait GameBoard {
    type Item: ToProtobuf;
    fn get_content(&self) -> Vec<Vec<Self::Item>>;
}

impl<T, Row: WithLength, Col: WithLength> GameBoard for Grid<T, Row, Col>
where
    T: Clone + ToProtobuf
{
    type Item = T;

    fn get_content(&self) -> Vec<Vec<Self::Item>> {
        self.iter()
            .map(|row| row.iter().cloned().collect())
            .collect()
    }
}

pub trait Game: Sized {
    type TurnData: FromProtobuf;
    type Players: PlayerQueue;
    type Board: GameBoard;

    fn new(players: &[PlayerId]) -> GameResult<Self>;
    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState>;

    fn board(&self) -> &Self::Board;

    fn players(&self) -> &Self::Players;
    fn players_mut(&mut self) -> &mut Self::Players;

    fn state(&self) -> GameState;
    fn set_state(&mut self, state: GameState);

    fn is_finished(&self) -> bool {
        matches!(self.state(), GameState::Finished(_))
    }

    fn set_draw(&mut self) -> GameState {
        self.set_state(GameState::Finished(FinishedState::Draw));
        self.state()
    }

    fn set_winner(&mut self, id: PlayerId) -> GameState {
        self.set_state(GameState::Finished(FinishedState::Win(id)));
        self.state()
    }

    fn get_board_content(&self) -> Vec<Vec<<Self::Board as GameBoard>::Item>> {
        self.board().get_content()
    }

    fn get_current_player(&mut self) -> GameResult<&<Self::Players as PlayerQueue>::Item> {
        self.players_mut()
            .get_current()
            .ok_or(GameError::PlayerPoolCorrupted)
    }
    fn get_enemy_player(&mut self) -> GameResult<&<Self::Players as PlayerQueue>::Item> {
        let current_id = self.get_current_player()?.get_id();
        self.players()
            .find(|p| p.get_id() != current_id)
            .ok_or(GameError::PlayerPoolCorrupted)
    }

    fn get_player_ids(&self) -> Vec<PlayerId> {
        self.players()
            .get_all()
            .iter()
            .map(|p| p.get_id())
            .collect()
    }

    fn switch_player(&mut self) -> GameResult<GameState> {
        let next_player = self
            .players_mut()
            .next()
            .ok_or(GameError::PlayerPoolCorrupted)?
            .get_id();
        self.set_state(GameState::Turn(next_player));
        Ok(self.state())
    }
}
