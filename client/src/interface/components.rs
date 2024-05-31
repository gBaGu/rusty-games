use crate::app_state::{AppState, AppStateTransition};
use bevy::prelude::*;
use bevy_simple_text_input::{TextInputBundle, TextInputTextStyle};

use crate::game::GameInfo;

#[derive(Clone, Debug, Deref, Component)]
pub struct JoinGame(pub GameInfo);

#[derive(Debug, Component)]
pub struct SubmitButton {
    pub source: Entity,
}

#[derive(Component)]
pub struct CreateGame;

#[derive(Component)]
pub struct Overlay;

// Bundles

#[derive(Bundle)]
pub struct MenuNavigationButtonBundle {
    pub button: ButtonBundle,
    pub state_transition: AppStateTransition,
}

impl MenuNavigationButtonBundle {
    pub fn new(style: Style, state: AppState) -> Self {
        Self {
            button: ButtonBundle { style, ..default() },
            state_transition: AppStateTransition(Some(state)),
        }
    }

    pub fn exit(style: Style) -> Self {
        Self {
            button: ButtonBundle { style, ..default() },
            state_transition: AppStateTransition(None),
        }
    }
}

#[derive(Bundle)]
pub struct OverlayNodeBundle {
    pub node: NodeBundle,
    pub tag: Overlay,
}

impl Default for OverlayNodeBundle {
    fn default() -> Self {
        Self {
            node: NodeBundle {
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
            tag: Overlay,
        }
    }
}

#[derive(Bundle)]
pub struct JoinGameButtonBundle {
    pub button: ButtonBundle,
    pub join: JoinGame,
}

impl JoinGameButtonBundle {
    pub fn new(style: Style, game: GameInfo) -> Self {
        Self {
            button: ButtonBundle { style, ..default() },
            join: JoinGame(game),
        }
    }
}

#[derive(Bundle)]
pub struct TextInputNodeBundle {
    pub node: NodeBundle,
    pub input: TextInputBundle,
}

impl TextInputNodeBundle {
    pub fn new(style: Style, text_style: TextStyle) -> Self {
        Self {
            node: NodeBundle { style, ..default() },
            input: TextInputBundle {
                text_style: TextInputTextStyle(text_style),
                ..default()
            },
        }
    }
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
