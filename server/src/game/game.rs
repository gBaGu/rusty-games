use std::num::TryFromIntError;

use crate::game::error::GameError;
use crate::game::player_pool::{PlayerId, PlayerQueue, WithPlayerId};

pub type GameResult<T> = Result<T, GameError>;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum FromProtobufError {
    #[error("invalid row (expected: 1-{max_expected}, found: {found})")]
    InvalidGridRow { max_expected: usize, found: usize },
    #[error("invalid column (expected: 1-{max_expected}, found: {found})")]
    InvalidGridCol { max_expected: usize, found: usize },
    #[error("protobuf message is invalid: {reason}")]
    InvalidProtobufMessage { reason: String },
    #[error("message data has missing field: {missing_field}")]
    MessageDataMissing { missing_field: String },
    #[error(transparent)]
    TurnDataConversion(#[from] TryFromIntError),
    #[error(transparent)]
    ProstDecodeError(#[from] prost::DecodeError),
}

pub trait FromProtobuf: Sized {
    fn from_protobuf(buf: &[u8]) -> Result<Self, FromProtobufError>;
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

pub trait Game: Sized {
    type TurnData: FromProtobuf;
    type Players: PlayerQueue;

    fn new(players: &[PlayerId]) -> GameResult<Self>;
    fn update(&mut self, id: PlayerId, data: Self::TurnData) -> GameResult<GameState>;

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
