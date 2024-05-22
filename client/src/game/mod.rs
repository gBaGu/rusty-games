mod error;

use crate::grpc::proto;
use game_server::game::game::{FinishedState, GameState};

pub struct GameInfo {
    id: u64,
    players: Vec<u64>,
    state: GameState,
}

impl TryFrom<proto::GameInfo> for GameInfo {
    type Error = error::GameError;

    fn try_from(value: proto::GameInfo) -> Result<Self, error::GameError> {
        let state = value
            .game_state
            .ok_or(error::GameError::InvalidProtobufMessage {
                reason: "missing game_state".to_string(),
            })?;
        let state = match (state.next_player_id, state.winner) {
            (Some(next), None) => GameState::Turn(next),
            (None, Some(winner)) => GameState::Finished(FinishedState::Win(winner)),
            (None, None) => GameState::Finished(FinishedState::Draw),
            _ => {
                return Err(error::GameError::InvalidProtobufMessage {
                    reason: "invalid game_state".to_string(),
                })
            }
        };
        Ok(Self {
            id: value.game_id,
            players: value.players,
            state,
        })
    }
}
