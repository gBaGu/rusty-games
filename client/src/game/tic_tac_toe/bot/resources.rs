use std::path::Path;

use bevy::asset::io::file::FileAssetReader;
use bevy::prelude::*;
use game_server::core;
use tic_tac_toe_ai::Agent;

use super::TTTBoard;
use crate::game::BotDifficulty;

pub const Q_LEARNING_AGENT_PATH: &str = "assets/agents/";

/// Resource that stores Q-Learning model weights for each available difficulty.
#[derive(Resource)]
pub struct QLearningModel {
    easy: Option<Agent>,
    medium: Option<Agent>,
    hard: Option<Agent>,
}

impl QLearningModel {
    pub fn load() -> Self {
        let parent_dir = FileAssetReader::get_base_path().join(Q_LEARNING_AGENT_PATH);
        let easy = Self::load_difficulty(&parent_dir, BotDifficulty::Easy);
        let medium = Self::load_difficulty(&parent_dir, BotDifficulty::Medium);
        let hard = Self::load_difficulty(&parent_dir, BotDifficulty::Hard);
        Self { easy, medium, hard }
    }

    pub fn load_difficulty<P: AsRef<Path>>(
        parent_dir: P,
        difficulty: BotDifficulty,
    ) -> Option<Agent> {
        let path = parent_dir
            .as_ref()
            .join(format!("{}.weights", difficulty.filename()));
        match Agent::load(&path) {
            Ok(agent) => Some(agent),
            Err(err) => {
                println!(
                    "failed to load {} q-learning model from {:?}: {}",
                    difficulty.filename(),
                    path,
                    err,
                );
                None
            }
        }
    }

    pub fn get_move(&self, difficulty: BotDifficulty, board: &TTTBoard) -> Option<core::GridIndex> {
        let agent = match difficulty {
            BotDifficulty::Easy => self.easy.as_ref(),
            BotDifficulty::Medium => self.medium.as_ref(),
            BotDifficulty::Hard => self.hard.as_ref(),
        };
        agent.and_then(|agent| {
            agent
                .get_best_action(&board)
                .and_then(|action| Some(action.into()))
        })
    }
}
