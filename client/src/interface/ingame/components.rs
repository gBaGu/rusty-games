use bevy::prelude::*;

use super::ITEM_HEIGHT;

#[derive(Component)]
pub struct InGameUI {
    pub player_id: u64,
    pub enemy_id: u64,
}

#[derive(Component)]
pub struct NextPlayer;

/// Contains player id
#[derive(Debug, Component)]
pub struct PlayerInfo {
    pub id: u64,
    pub color: Color,
    pub image: Option<Handle<Image>>,
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
    pub fn new(player_id: u64, enemy_id: u64) -> Self {
        Self {
            node: NodeBundle {
                style: Style {
                    display: Display::Flex,
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    ..default()
                },
                ..default()
            },
            in_game_ui: InGameUI {
                player_id,
                enemy_id,
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
    pub fn new(id: u64, color: Color) -> Self {
        Self {
            node: NodeBundle {
                style: Style {
                    display: Display::Flex,
                    flex_basis: Val::Percent(100.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    border: UiRect::all(Val::Px(1.0)),
                    ..default()
                },
                ..default()
            },
            info: PlayerInfo {
                id,
                color,
                image: None,
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
