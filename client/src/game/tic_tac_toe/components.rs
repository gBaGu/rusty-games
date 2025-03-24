use bevy::prelude::*;
use game_server::core::tic_tac_toe::TicTacToe;
use game_server::core::Game as _;

use super::bot::Strategy;
use crate::game::{BotDifficulty, GameInfo};

/// This enum is used to specify if the opponent is a bot or another user.
#[derive(Clone, Copy, Debug)]
pub enum EnemyType {
    User(u64),
    Bot {
        strategy: Strategy,
        difficulty: Option<BotDifficulty>,
    },
}

/// Component that is used in combination with GameDataReady event
/// to create Tic Tac Toe game entity.
#[derive(Clone, Debug, Component)]
pub struct CreateGameContext {
    enemy: EnemyType,
    user_first: bool,
    game: Option<TicTacToe>,
}

impl CreateGameContext {
    pub fn new(enemy: EnemyType, user_first: bool, game: Option<TicTacToe>) -> Self {
        Self {
            enemy,
            user_first,
            game,
        }
    }

    pub fn new_from_game_info(user: u64, game_info: GameInfo) -> Option<Self> {
        let (enemy, user_first) = if game_info.get_user_id(0)? == user {
            (game_info.get_user_id(1)?, true)
        } else if game_info.get_user_id(1)? == user {
            (game_info.get_user_id(0)?, false)
        } else {
            return None;
        };
        let mut ttt = TicTacToe::default();
        ttt.set_state(game_info.state);
        if let Some(board) = game_info.board {
            ttt.set_board(board);
        }
        Some(Self::new(EnemyType::User(enemy), user_first, Some(ttt)))
    }

    pub fn enemy(&self) -> EnemyType {
        self.enemy
    }

    pub fn user_first(&self) -> bool {
        self.user_first
    }

    pub fn game(&self) -> Option<&TicTacToe> {
        self.game.as_ref()
    }
}
