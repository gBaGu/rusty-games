use bevy::prelude::{Bundle, Component, NodeBundle};

use crate::game::GameInfo;
use crate::grpc::CallGetPlayerGames;

#[derive(Component, Default)]
pub struct GameList {
    pub games: Vec<GameInfo>
}

#[derive(Bundle)]
pub struct GameListBundle {
    pub container: NodeBundle,
    pub games: GameList,
}

#[derive(Bundle)]
pub struct LoadingGameListBundle {
    pub container: NodeBundle,
    pub games: GameList,
    pub task: CallGetPlayerGames,
}
