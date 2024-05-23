tonic::include_proto!("game");

use crate::game::game;

pub const FILE_DESCRIPTOR_SET: &[u8] = tonic::include_file_descriptor_set!("game_descriptor");

impl From<game::GameState> for GameState {
    fn from(value: game::GameState) -> Self {
        match value {
            game::GameState::Turn(id) => Self {
                next_player_id: Some(id),
                ..Default::default()
            },
            game::GameState::Finished(game::FinishedState::Win(id)) => Self {
                winner: Some(id),
                ..Default::default()
            },
            game::GameState::Finished(game::FinishedState::Draw) => Self::default(),
        }
    }
}
