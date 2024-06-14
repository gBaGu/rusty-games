use bevy::prelude::*;
use bevy_simple_text_input::{TextInputBundle, TextInputTextStyle};

use crate::app_state::{AppState, AppStateTransition};
use crate::game::GameInfo;
use crate::interface::common::{PRIMARY_COLOR, OVERLAY_BACKGROUND_COLOR};

#[derive(Clone, Debug, Deref, Component)]
pub struct JoinGame(pub GameInfo);

#[derive(Debug, Component)]
pub struct SubmitButton {
    pub source: Entity,
}

/// Tag type to mark input components that they are used to set setting
#[derive(Debug, Component)]
pub enum Setting {
    UserId,
}

/// Tag type to mark input components that they are used to create game
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
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            state_transition: AppStateTransition(Some(state)),
        }
    }

    pub fn exit(style: Style) -> Self {
        Self {
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
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
                background_color: OVERLAY_BACKGROUND_COLOR.into(),
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
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            join: JoinGame(game),
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
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            submit: SubmitButton { source },
        }
    }
}

#[derive(Bundle)]
pub struct SettingTextInputBundle {
    pub node: NodeBundle,
    pub text_input: TextInputBundle,
    pub setting: Setting,
}

impl SettingTextInputBundle {
    pub fn new(style: Style, text_style: TextStyle, setting: Setting) -> Self {
        Self {
            node: NodeBundle { style, ..default() },
            text_input: TextInputBundle {
                text_style: TextInputTextStyle(text_style),
                ..default()
            },
            setting,
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
    pub fn new(style: Style, text_style: TextStyle) -> Self {
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
