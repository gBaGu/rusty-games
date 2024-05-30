use bevy::ecs::{component::Component, entity::Entity};
use bevy::prelude::{default, Bundle, Color, JustifyContent, NodeBundle, Val};
use bevy::ui::{AlignItems, BackgroundColor, Display, Style};

#[derive(Debug, Component)]
pub struct AssociatedTextInput(pub Entity);

#[derive(Debug, Component)]
pub struct Overlay;

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
