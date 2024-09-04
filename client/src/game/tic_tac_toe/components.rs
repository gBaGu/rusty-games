use bevy::prelude::*;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::Game;

use super::bot::Strategy;
use crate::game::{BotDifficulty, FullGameInfo, GameInfo};

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
        Some(Self {
            enemy: EnemyType::User(enemy),
            user_first,
            game: None,
        })
    }

    pub fn new_from_full_game_info(user: u64, game_info: FullGameInfo) -> Option<Self> {
        let FullGameInfo { info, board } = game_info;
        let mut ttt = TicTacToe::default();
        ttt.set_state(info.state);
        ttt.set_board(board);
        Self::new_from_game_info(user, info).and_then(|mut ctx| {
            ctx.game = Some(ttt);
            Some(ctx)
        })
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
