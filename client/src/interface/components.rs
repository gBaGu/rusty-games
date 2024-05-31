use bevy::ecs::{component::Component, entity::Entity};
use bevy::prelude::{
    default, Bundle, ButtonBundle, Color, JustifyContent, NodeBundle, TextStyle, Val,
};
use bevy::ui::{AlignItems, BackgroundColor, Display, Style};
use bevy_simple_text_input::{TextInputBundle, TextInputTextStyle};

#[derive(Debug, Component)]
pub struct SubmitButton {
    pub source: Entity,
}

#[derive(Component)]
pub struct CreateGame;

#[derive(Debug, Component)]
pub struct Overlay;

// Bundles

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

#[derive(Bundle)]
pub struct SubmitButtonBundle {
    pub button: ButtonBundle,
    pub submit: SubmitButton,
}

impl SubmitButtonBundle {
    pub fn new(style: Style, source: Entity) -> Self {
        Self {
            button: ButtonBundle { style, ..default() },
            submit: SubmitButton { source },
        }
    }
}

#[derive(Bundle)]
pub struct NetworkGameTextInputBundle {
    pub node: NodeBundle,
    pub text_input: TextInputBundle,
    pub tag: CreateGame,
}

impl NetworkGameTextInputBundle {
    pub fn new(text_style: TextStyle, style: Style) -> Self {
        Self {
            node: NodeBundle { style, ..default() },
            text_input: TextInputBundle {
                text_style: TextInputTextStyle(text_style),
                ..default()
            },
            tag: CreateGame,
        }
    }
}
