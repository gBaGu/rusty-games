use bevy::prelude::*;
use game_server::core;

use super::ITEM_HEIGHT;
use crate::game::{GameLink, PlayerPosition};
use crate::interface;

pub fn ui_info_node() -> Node {
    Node {
        flex_basis: Val::Percent(100.0),
        border: UiRect::all(Val::Px(2.0)),
        ..interface::common::flex_row()
    }
}

fn image_node() -> Node {
    Node {
        width: Val::Px(ITEM_HEIGHT),
        height: Val::Px(ITEM_HEIGHT),
        margin: UiRect::all(Val::Px(10.0)),
        ..default()
    }
}

/// Component that indicates the container with game state information.
#[derive(Debug, Component)]
pub struct GameStateBox;

/// Component that indicates the image of a next player sign.
#[derive(Debug, Component)]
pub struct NextPlayer;

#[derive(Debug, Bundle)]
pub struct GameStateInfoBundle {
    node: Node,
    game_link: GameLink,
    game_state_box: GameStateBox,
}

impl GameStateInfoBundle {
    pub fn new(game: Entity) -> Self {
        Self {
            node: ui_info_node(),
            game_link: game.into(),
            game_state_box: GameStateBox,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct PlayerInfoBundle {
    node: Node,
    border_color: BorderColor,
    game_link: GameLink,
    player: PlayerPosition,
    color: interface::PlayerColor,
}

impl PlayerInfoBundle {
    pub fn new_active(game: Entity, player: core::PlayerPosition, color: Color) -> Self {
        Self {
            node: ui_info_node(),
            border_color: color.into(),
            game_link: game.into(),
            player: PlayerPosition::new(player),
            color: color.into(),
        }
    }

    pub fn new_inactive(game: Entity, player: core::PlayerPosition, color: Color) -> Self {
        Self {
            node: ui_info_node(),
            border_color: Default::default(),
            game_link: game.into(),
            player: PlayerPosition::new(player),
            color: color.into(),
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct PlayerImageBundle {
    node: Node,
    image: ImageNode,
}

impl PlayerImageBundle {
    pub fn new(image: Handle<Image>) -> Self {
        Self {
            node: image_node(),
            image: ImageNode::new(image),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NextPlayerImageBundle {
    node: Node,
    game_link: GameLink,
    image: ImageNode,
    next_player: NextPlayer,
}

impl NextPlayerImageBundle {
    pub fn new(game: Entity, img: Handle<Image>) -> Self {
        Self {
            node: image_node(),
            game_link: game.into(),
            image: ImageNode::new(img),
            next_player: NextPlayer,
        }
    }

    #[allow(dead_code)]
    pub fn empty(game: Entity) -> Self {
        Self::new(game, Handle::default())
    }
}
