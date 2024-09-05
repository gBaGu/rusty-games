use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_simple_text_input::{TextInputBundle, TextInputTextStyle};

use crate::app_state::{AppState, AppStateTransition};
use crate::game::{GameInfo, GameLink};
use crate::interface::common::{column_node_bundle, OVERLAY_BACKGROUND_COLOR, PRIMARY_COLOR};

/// Component that indicates that the game is being shawn on the screen.
/// Board and in-game ui will be connected to this component.
#[derive(Debug, Component)]
pub struct Playground;

/// Component that indicates that the ui node contains game settings.
/// This container will be filled depending on a current game page and app state.
#[derive(Debug, Component)]
pub struct GameSettings;

/// Component that indicates that entity is related to a particular game type T.
#[derive(Debug, Component)]
pub struct GameTag<T>(PhantomData<T>);

impl<T> Default for GameTag<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

/// Component that indicates that entity is related to a settings entity.
#[derive(Debug, Component)]
pub struct GameSettingsLink(Entity);

impl GameSettingsLink {
    pub fn new(settings: Entity) -> Self {
        Self(settings)
    }

    pub fn get(&self) -> Entity {
        self.0
    }
}

/// Component that stores color that will be used in in-game ui to identify a player.
#[derive(Clone, Copy, Debug, Deref, DerefMut, Component)]
pub struct PlayerColor(Color);

impl From<Color> for PlayerColor {
    fn from(value: Color) -> Self {
        Self(value)
    }
}

/// Component that stores information about a game that can be joined.
#[derive(Clone, Debug, Deref, Component)]
pub struct JoinGame(pub GameInfo);

/// Component that indicates that button entity is used to submit some information.
#[derive(Debug, Component)]
pub struct SubmitButton {
    pub source: Entity,
}

/// Tag type to mark input components that they are used to set setting
#[derive(Debug, Component)]
pub enum Setting {
    UserId,
}

/// Component that indicates that input entity is used to create user id value.
#[derive(Component)]
pub struct UserIdInput;

/// Tag type to mark input components that they are used to create game
#[derive(Debug, Component)]
pub struct CreateGame;

/// Component that indicates that the entity should be drawn on top of everything else.
#[derive(Component)]
pub struct Overlay;

// Bundles

#[derive(Debug, Bundle)]
pub struct PlaygroundBundle {
    pub node: NodeBundle,
    pub game_link: GameLink,
    pub playground: Playground,
}

#[derive(Debug, Bundle)]
pub struct GameSettingsBundle {
    pub node: NodeBundle,
    pub game_settings: GameSettings,
}

impl GameSettingsBundle {
    pub fn new() -> Self {
        Self {
            node: column_node_bundle(),
            game_settings: GameSettings,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct GamePageButtonBundle<T: Send + Sync + 'static> {
    pub button: ButtonBundle,
    pub game_tag: GameTag<T>,
}

impl<T: Send + Sync + 'static> GamePageButtonBundle<T> {
    pub fn new(style: Style) -> Self {
        Self {
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            game_tag: GameTag::default(),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct CreateGameButtonBundle {
    pub button: ButtonBundle,
    pub game_settings_link: GameSettingsLink,
    pub create_game: CreateGame,
}

impl CreateGameButtonBundle {
    pub fn new(style: Style, settings: Entity) -> Self {
        Self {
            button: ButtonBundle {
                style,
                background_color: PRIMARY_COLOR.into(),
                ..default()
            },
            game_settings_link: GameSettingsLink(settings),
            create_game: CreateGame,
        }
    }
}

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
pub struct UserIdTextInputBundle {
    pub node: NodeBundle,
    pub text_input: TextInputBundle,
    pub user_id_input: UserIdInput,
}

impl UserIdTextInputBundle {
    pub fn new(style: Style, text_style: TextStyle) -> Self {
        Self {
            node: NodeBundle { style, ..default() },
            text_input: TextInputBundle {
                text_style: TextInputTextStyle(text_style),
                ..default()
            },
            user_id_input: UserIdInput,
        }
    }
}

#[derive(Bundle)]
pub struct UiImageBundle {
    pub node: NodeBundle,
    pub image: UiImage,
}

impl UiImageBundle {
    pub fn new(style: Style, image: Handle<Image>) -> Self {
        Self {
            node: NodeBundle {
                style,
                ..default()
            },
            image: UiImage::new(image),
        }
    }
}
