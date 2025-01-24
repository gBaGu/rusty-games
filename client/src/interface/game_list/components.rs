use bevy::prelude::*;

use crate::game::GameInfo;
use crate::interface::common::column_node;

#[derive(Component)]
pub enum GameList {
    Games(Vec<GameInfo>),
    Message(String),
}

impl Default for GameList {
    fn default() -> Self {
        Self::Message("Loading...".into())
    }
}

// Bundles

#[derive(Bundle)]
pub struct GameListBundle {
    node: Node,
    list: GameList,
}

impl Default for GameListBundle {
    fn default() -> Self {
        let mut node = column_node();
        node.width = Val::Percent(80.);
        Self {
            node,
            list: GameList::default(),
        }
    }
}

impl GameListBundle {
    pub fn node_mut(&mut self) -> &mut Node {
        &mut self.node
    }
}
