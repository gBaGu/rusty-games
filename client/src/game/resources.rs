use bevy::prelude::*;
use game_server::game::tic_tac_toe::TicTacToe;
use game_server::game::{BoardCell, GameState, PlayerId as GamePlayerId};

use super::error::GameError;
use super::{
    GameInfo, LocalGameTurn, NetworkGameTurn, Position, BOARD_SIZE, GAME_REFRESH_INTERVAL_SEC,
    O_SPRITE_PATH, X_SPRITE_PATH,
};

#[derive(Deref, DerefMut, Resource)]
pub struct RefreshGameTimer(pub Timer);

impl Default for RefreshGameTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_REFRESH_INTERVAL_SEC,
            TimerMode::Repeating,
        ))
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameType {
    Network(u64),
    Local,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Authority {
    Player(u64),
    Bot(u64),
}

#[derive(Clone, Debug)]
pub struct PlayerData {
    auth: Authority,
    game_player_id: GamePlayerId,
    image: Handle<Image>,
}

impl PlayerData {
    pub fn new_player(id: u64, game_player_id: GamePlayerId, image: Handle<Image>) -> Self {
        Self {
            auth: Authority::Player(id),
            game_player_id,
            image,
        }
    }

    pub fn new_bot(id: u64, game_player_id: GamePlayerId, image: Handle<Image>) -> Self {
        Self {
            auth: Authority::Bot(id),
            game_player_id,
            image,
        }
    }

    pub fn auth(&self) -> Authority {
        self.auth
    }

    pub fn game_player_id(&self) -> GamePlayerId {
        self.game_player_id
    }

    pub fn image(&self) -> &Handle<Image> {
        &self.image
    }
}

#[derive(Debug, Resource)]
pub struct CurrentGame {
    game_type: GameType,
    user_data: PlayerData,
    enemy_data: PlayerData,
    state: GameState,
    board: [[BoardCell<GamePlayerId>; BOARD_SIZE]; BOARD_SIZE],
    board_entity: Option<Entity>,
}

impl CurrentGame {
    fn new(
        game_type: GameType,
        user_data: PlayerData,
        enemy_data: PlayerData,
        state: GameState,
    ) -> Self {
        Self {
            game_type,
            user_data,
            enemy_data,
            state,
            board: Default::default(),
            board_entity: None,
        }
    }

    pub fn new_over_network(
        user_id: u64,
        game: GameInfo,
        asset_server: &AssetServer,
    ) -> Result<Self, GameError> {
        let x_img = asset_server.load(X_SPRITE_PATH);
        let o_img = asset_server.load(O_SPRITE_PATH);
        let player1 = PlayerData::new_player(game.players[0], 0, x_img);
        let player2 = PlayerData::new_player(game.players[1], 1, o_img);
        let (user, enemy) = if game.players[0] == user_id {
            (player1, player2)
        } else if game.players[1] == user_id {
            (player2, player1)
        } else {
            return Err(GameError::ForeignGame {
                user: user_id,
                game: game.id,
            });
        };
        Ok(Self::new(
            GameType::Network(game.id),
            user,
            enemy,
            game.state,
        ))
    }

    pub fn new_with_bot(
        user_id: u64,
        bot_id: u64,
        user_first: bool,
        state: GameState,
        board: [[BoardCell<GamePlayerId>; BOARD_SIZE]; BOARD_SIZE],
        asset_server: &AssetServer,
    ) -> Self {
        let x_img = asset_server.load(X_SPRITE_PATH);
        let o_img = asset_server.load(O_SPRITE_PATH);
        let (user, bot) = if user_first {
            (
                PlayerData::new_player(user_id, 0, x_img),
                PlayerData::new_bot(bot_id, 1, o_img),
            )
        } else {
            (
                PlayerData::new_player(user_id, 1, o_img),
                PlayerData::new_bot(bot_id, 0, x_img),
            )
        };
        let mut game = Self::new(GameType::Local, user, bot, state);
        game.board = board;
        game
    }

    pub fn game_type(&self) -> GameType {
        self.game_type
    }

    pub fn user_data(&self) -> &PlayerData {
        &self.user_data
    }

    pub fn enemy_data(&self) -> &PlayerData {
        &self.enemy_data
    }

    pub fn state(&self) -> GameState {
        self.state
    }

    pub fn board(&self) -> &[[BoardCell<GamePlayerId>; BOARD_SIZE]] {
        &self.board
    }

    pub fn board_entity(&self) -> &Option<Entity> {
        &self.board_entity
    }

    pub fn get_next_player(&self) -> Option<&PlayerData> {
        if let GameState::Turn(id) = self.state {
            if self.user_data.game_player_id == id {
                return Some(&self.user_data);
            } else if self.enemy_data.game_player_id == id {
                return Some(&self.enemy_data);
            }
        }
        None
    }

    pub fn get_player_image(&self, id: GamePlayerId) -> Option<&Handle<Image>> {
        if self.user_data.game_player_id == id {
            return Some(&self.user_data.image);
        } else if self.enemy_data.game_player_id == id {
            return Some(&self.enemy_data.image);
        } else {
            None
        }
    }

    pub fn get_cell(&self, pos: (usize, usize)) -> BoardCell<GamePlayerId> {
        self.board[pos.0][pos.1]
    }

    pub fn set_board(&mut self, board: Entity) {
        self.board_entity = Some(board);
    }

    pub fn set_cell(&mut self, pos: (usize, usize), player_id: GamePlayerId) {
        self.board[pos.0][pos.1] = player_id.into()
    }

    pub fn set_state(&mut self, state: GameState) {
        self.state = state;
    }

    pub fn trigger_turn(
        &self,
        network_turn_data: &mut EventWriter<NetworkGameTurn>,
        local_turn_data: &mut EventWriter<LocalGameTurn>,
        auth: Authority,
        pos: Position,
    ) {
        match self.game_type {
            GameType::Network(id) => {
                network_turn_data.send(NetworkGameTurn {
                    game_id: id,
                    auth,
                    pos,
                });
            }
            GameType::Local => {
                local_turn_data.send(LocalGameTurn { auth, pos });
            }
        };
    }
}

#[derive(Debug, Default, Deref, DerefMut, Resource)]
pub struct LocalGame(TicTacToe);
