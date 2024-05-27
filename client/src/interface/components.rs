use bevy::ecs::{component::Component, entity::Entity};
use bevy::prelude::{default, Bundle, Color, JustifyContent, NodeBundle, Val};
use bevy::ui::{AlignItems, BackgroundColor, Display, Style, UiRect};

#[derive(Debug, Component)]
pub struct AssociatedTextInput(pub Entity);

#[derive(Debug, Component)]
pub struct NextPlayerImage;

#[derive(Debug, Component)]
pub struct Overlay;

#[derive(Bundle, Debug)]
pub struct EmptyNextPlayerImageBundle {
    node_bundle: NodeBundle,
    next_player_image: NextPlayerImage,
}

pub fn empty_next_player_image(size: Val) -> EmptyNextPlayerImageBundle {
    EmptyNextPlayerImageBundle {
        node_bundle: NodeBundle {
            style: Style {
                width: size,
                height: size,
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            ..default()
        },
        next_player_image: NextPlayerImage,
    }
}

pub fn overlay_ui_node() -> impl Bundle {
    (
        NodeBundle {
            style: Style {
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                height: Val::Percent(100.0),
                width: Val::Percent(100.0),
                ..default()
            },
            background_color: BackgroundColor(Color::DARK_GRAY.with_a(0.95)),
            ..default()
        },
        Overlay,
    )
}
