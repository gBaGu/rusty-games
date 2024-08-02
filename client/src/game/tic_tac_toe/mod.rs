mod board;
pub mod bot;
mod resources;
mod systems;
mod ui;

use bevy::prelude::*;
use game_server::game::grid::GridIndex;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::Game;

use crate::game::{EnemyType, FullGameInfo, GameInfo};
use crate::grpc::NetworkSystems;
use resources::Images;
use systems::*;

pub const X_SPRITE_PATH: &str = "sprites/X.png";
pub const O_SPRITE_PATH: &str = "sprites/O.png";

pub type TTTBoard = <TicTacToe as Game>::Board;

type CreateGame = super::CreateGame<CreateGameContext>;
type LocalGame = super::LocalGame<TicTacToe>;
type LocalGameBundle = super::LocalGameBundle<TicTacToe>;
type NetworkGameBundle = super::NetworkGameBundle<TicTacToe>;
type PendingAction = super::PendingAction<GridIndex>;
type PendingActionBundle = super::PendingActionBundle<GridIndex>;
type PlayerActionInitialized = super::PlayerActionInitialized<GridIndex>;
type PlayerActionApplied = super::PlayerActionApplied<GridIndex>;

#[derive(Clone, Debug)]
pub struct CreateGameContext {
    user: u64,
    enemy: EnemyType<bot::Strategy>,
    user_first: bool,
    game: Option<TicTacToe>,
}

impl CreateGameContext {
    pub fn new(
        user: u64,
        enemy: EnemyType<bot::Strategy>,
        user_first: bool,
        game: Option<TicTacToe>,
    ) -> Self {
        Self {
            user,
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
            user,
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

    pub fn user(&self) -> u64 {
        self.user
    }

    pub fn enemy(&self) -> EnemyType<bot::Strategy> {
        self.enemy
    }

    pub fn user_first(&self) -> bool {
        self.user_first
    }
}

pub struct TicTacToePlugin;

impl Plugin for TicTacToePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((board::BoardPlugin, bot::BotPlugin, ui::InGameUIPlugin))
            .add_event::<CreateGame>()
            .add_event::<PlayerActionInitialized>()
            .add_event::<PlayerActionApplied>()
            .add_systems(Startup, init)
            .add_systems(
                Update,
                (
                    handle_create_game_reply,
                    handle_get_game_reply_on_join,
                    handle_get_game,
                    handle_make_turn,
                    create,
                    create_pending_action,
                    apply_action,
                    (send_pending_action, send_get_game).in_set(NetworkSystems),
                ),
            );
    }
}
