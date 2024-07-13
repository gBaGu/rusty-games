use bevy::prelude::*;
use game_server::game::PlayerId as PlayerPosition;

use super::ITEM_HEIGHT;
use crate::game::Authority;

#[derive(Component)]
pub struct InGameUI {
    pub player: Authority,
    pub player_position: PlayerPosition,
    pub player_image: Handle<Image>,
    pub enemy: Authority,
    pub enemy_position: PlayerPosition,
    pub enemy_image: Handle<Image>,
}

#[derive(Component)]
pub struct NextPlayer;

/// Contains player id
#[derive(Debug, Component)]
pub struct PlayerInfo {
    pub position: PlayerPosition,
    pub color: Color,
    pub image: Handle<Image>,
}

#[derive(Component)]
pub struct PlayerImage;

#[derive(Component)]
pub struct GameStateContainer;

// Bundles

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

#[derive(Bundle)]
pub struct InGameUIBundle {
    pub node: NodeBundle,
    pub in_game_ui: InGameUI,
}

impl InGameUIBundle {
    pub fn new(
        player_auth: Authority,
        player_position: PlayerPosition,
        player_image: Handle<Image>,
        enemy_auth: Authority,
        enemy_position: PlayerPosition,
        enemy_image: Handle<Image>,
    ) -> Self {
        Self {
            node: NodeBundle {
                style: Style {
                    display: Display::Flex,
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    margin: UiRect::bottom(Val::Auto),
                    ..default()
                },
                ..default()
            },
            in_game_ui: InGameUI {
                player: player_auth,
                player_position,
                player_image,
                enemy: enemy_auth,
                enemy_position,
                enemy_image,
            },
        }
    }
}

#[derive(Bundle)]
pub struct PlayerInfoContainerBundle {
    pub node: NodeBundle,
    pub info: PlayerInfo,
}

impl PlayerInfoContainerBundle {
    pub fn new(position: PlayerPosition, color: Color, image: Handle<Image>) -> Self {
        Self {
            node: NodeBundle {
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
            },
            info: PlayerInfo {
                position,
                color,
                image,
            },
        }
    }
}

#[derive(Bundle)]
pub struct PlayerImageBundle {
    pub node: NodeBundle,
    pub image: UiImage,
    pub tag: PlayerImage,
}

impl Default for PlayerImageBundle {
    fn default() -> Self {
        Self {
            node: image_node_bundle(),
            image: UiImage::default(),
            tag: PlayerImage,
        }
    }
}

impl PlayerImageBundle {
    pub fn new(image: Handle<Image>) -> Self {
        Self {
            node: image_node_bundle(),
            image: UiImage::new(image),
            tag: PlayerImage,
        }
    }
}

#[derive(Bundle)]
pub struct GameStateContainerBundle {
    pub node: NodeBundle,
    pub tag: GameStateContainer,
}

impl Default for GameStateContainerBundle {
    fn default() -> Self {
        Self {
            node: NodeBundle {
                style: Style {
                    display: Display::Flex,
                    flex_basis: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                },
                ..default()
            },
            tag: GameStateContainer,
        }
    }
}

#[derive(Bundle)]
pub struct EmptyNextPlayerImageBundle {
    pub node: NodeBundle,
    pub image: UiImage,
    pub tag: NextPlayer,
}

impl Default for EmptyNextPlayerImageBundle {
    fn default() -> Self {
        Self {
            node: image_node_bundle(),
            image: UiImage::default(),
            tag: NextPlayer,
        }
    }
}
