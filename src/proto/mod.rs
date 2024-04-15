tonic::include_proto!("game");
use crate::game::state::{FinishedState, GameState};

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");

impl MakeTurnReply {
    pub fn from_game_state(state: GameState) -> Self {
        match state {
            GameState::Turn(id) => Self {
                next_player_id: Some(id),
                ..Default::default()
            },
            GameState::Finished(FinishedState::Win(id)) => Self {
                winner: Some(id),
                ..Default::default()
            },
            GameState::Finished(FinishedState::Draw) => Self::default(),
        }
    }
}
