use bevy::prelude::*;
use game_server::game::PlayerId;

use super::ITEM_HEIGHT;
use crate::game::{GameLink, PlayerPosition};
use crate::interface::PlayerColor;

pub fn ui_info_container() -> NodeBundle {
    NodeBundle {
        style: Style {
            display: Display::Flex,
            flex_basis: Val::Percent(100.0),
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        ..default()
    }
}

fn image_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            width: Val::Px(ITEM_HEIGHT),
            height: Val::Px(ITEM_HEIGHT),
            margin: UiRect::all(Val::Px(10.0)),
            ..default()
        },
        background_color: Color::WHITE.into(),
        ..default()
    }
}

#[derive(Debug, Component)]
pub struct GameStateBox;

#[derive(Debug, Component)]
pub struct NextPlayer;

#[derive(Debug, Bundle)]
pub struct GameStateInfoBundle {
    node: NodeBundle,
    game_link: GameLink,
    game_state_box: GameStateBox,
}

impl GameStateInfoBundle {
    pub fn new(game: Entity) -> Self {
        Self {
            node: ui_info_container(),
            game_link: GameLink::new(game),
            game_state_box: GameStateBox,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct PlayerInfoBundle {
    node: NodeBundle,
    game_link: GameLink,
    player: PlayerPosition,
    color: PlayerColor,
}

impl PlayerInfoBundle {
    pub fn new_active(game: Entity, player: PlayerId, color: Color) -> Self {
        let mut node = ui_info_container();
        node.border_color = color.into();
        Self {
            node,
            game_link: GameLink::new(game),
            player: PlayerPosition::new(player),
            color: color.into(),
        }
    }

    pub fn new_inactive(game: Entity, player: PlayerId, color: Color) -> Self {
        Self {
            node: ui_info_container(),
            game_link: GameLink::new(game),
            player: PlayerPosition::new(player),
            color: color.into(),
        }
    }
}

#[derive(Debug, Default, Bundle)]
pub struct PlayerImageBundle {
    node: NodeBundle,
    image: UiImage,
}

impl PlayerImageBundle {
    pub fn new(image: Handle<Image>) -> Self {
        Self {
            node: image_node_bundle(),
            image: UiImage::new(image),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct NextPlayerImageBundle {
    node: NodeBundle,
    game_link: GameLink,
    image: UiImage,
    next_player: NextPlayer,
}

impl NextPlayerImageBundle {
    pub fn new(game: Entity, img: Handle<Image>) -> Self {
        Self {
            node: image_node_bundle(),
            game_link: GameLink::new(game),
            image: UiImage::new(img),
            next_player: NextPlayer,
        }
    }

    pub fn empty(game: Entity) -> Self {
        Self::new(game, Handle::default())
    }
}
