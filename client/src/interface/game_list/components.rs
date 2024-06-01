use bevy::prelude::*;

use crate::game::GameInfo;
use crate::interface::common::column_node_bundle;

#[derive(Component)]
pub enum GameList {
    Games(Vec<GameInfo>),
    Message(String),
}

impl Default for GameList {
    fn default() -> Self {
        Self::Games(vec![])
    }
}

// Bundles

#[derive(Bundle)]
pub struct GameListBundle {
    pub container: NodeBundle,
    pub list: GameList,
}

impl Default for GameListBundle {
    fn default() -> Self {
        Self {
            container: column_node_bundle(),
            list: GameList::default(),
        }
    }
}
