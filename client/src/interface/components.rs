use std::marker::PhantomData;

use bevy::prelude::*;
use bevy_simple_text_input::{TextInput, TextInputTextColor, TextInputTextFont};

use super::common::{self, OVERLAY_BACKGROUND_COLOR, PRIMARY_COLOR, SECONDARY_COLOR};
use crate::app_state::{AppState, AppStateTransition};
use crate::game::{GameInfo, GameLink};

/// Component that indicates that the game is being shawn on the screen.
/// Board and in-game ui will be connected to this component.
#[derive(Debug, Component)]
pub struct Playground;

/// Component that indicates that the ui node contains game settings.
/// This container will be filled depending on a current game page and app state.
#[derive(Debug, Component)]
pub struct GameSettingsContainer;

/// Component that indicates that entity is related to a particular game type T.
#[derive(Debug, Component)]
pub struct GameTag<T>(PhantomData<T>);

impl<T> Default for GameTag<T> {
    fn default() -> Self {
        Self(PhantomData::default())
    }
}

/// Marker component that indicates a button that is used to choose from multiple options.
#[derive(Debug, Component)]
pub struct SettingOption;

/// Points to entity that stores value chosen by interface component.
#[derive(Debug, Component)]
pub struct StorageLink(Entity);

impl StorageLink {
    pub fn new(setting: Entity) -> Self {
        Self(setting)
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

/// Tag type to mark input components that they are used to create game
#[derive(Debug, Component)]
pub struct CreateGame;

/// Component that indicates that the entity should be drawn on top of everything else.
#[derive(Component)]
pub struct Overlay;

// Bundles

#[derive(Debug, Bundle)]
pub struct PlaygroundBundle {
    node: Node,
    game_link: GameLink,
    playground: Playground,
}

impl PlaygroundBundle {
    pub fn new(game: Entity) -> Self {
        Self {
            node: common::root_node(),
            game_link: GameLink::new(game),
            playground: Playground,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct GameSettingsBundle {
    node: Node,
    game_settings: GameSettingsContainer,
}

impl GameSettingsBundle {
    pub fn new() -> Self {
        Self {
            node: common::column_node(),
            game_settings: GameSettingsContainer,
        }
    }
}

#[derive(Debug, Bundle)]
pub struct GamePageButtonBundle<T: Send + Sync + 'static> {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    game_tag: GameTag<T>,
}

impl<T: Send + Sync + 'static> GamePageButtonBundle<T> {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            game_tag: GameTag::default(),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct CreateGameButtonBundle {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    create_game: CreateGame,
}

impl CreateGameButtonBundle {
    pub fn new(node: Node) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            create_game: CreateGame,
        }
    }
}

#[derive(Bundle)]
pub struct MenuNavigationButtonBundle {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    state_transition: AppStateTransition,
}

impl MenuNavigationButtonBundle {
    pub fn new(node: Node, state: AppState) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            state_transition: AppStateTransition(Some(state)),
        }
    }

    pub fn exit(node: Node) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            state_transition: AppStateTransition(None),
        }
    }
}

#[derive(Bundle)]
pub struct OverlayNodeBundle {
    node: Node,
    background_color: BackgroundColor,
    tag: Overlay,
}

impl Default for OverlayNodeBundle {
    fn default() -> Self {
        Self {
            node: Node {
                display: Display::Flex,
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                height: Val::Percent(100.0),
                width: Val::Percent(100.0),
                ..default()
            },
            background_color: OVERLAY_BACKGROUND_COLOR.into(),
            tag: Overlay,
        }
    }
}

#[derive(Bundle)]
pub struct JoinGameButtonBundle {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    join: JoinGame,
}

impl JoinGameButtonBundle {
    pub fn new(node: Node, game: GameInfo) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            join: JoinGame(game),
        }
    }
}

#[derive(Bundle)]
pub struct SubmitButtonBundle {
    node: Node,
    background_color: BackgroundColor,
    button: Button,
    submit: SubmitButton,
}

impl SubmitButtonBundle {
    pub fn new(node: Node, source: Entity) -> Self {
        Self {
            node,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            submit: SubmitButton { source },
        }
    }
}

#[derive(Bundle)]
pub struct SettingTextInputBundle {
    node: Node,
    text_font: TextInputTextFont,
    text_color: TextInputTextColor,
    text_input: TextInput,
    setting: Setting,
}

impl SettingTextInputBundle {
    pub fn new(node: Node, text_font: TextFont, setting: Setting) -> Self {
        Self {
            node,
            text_font: TextInputTextFont(text_font),
            text_color: TextInputTextColor(SECONDARY_COLOR.into()),
            text_input: TextInput,
            setting,
        }
    }
}

#[derive(Bundle)]
pub struct TextInputBundle {
    node: Node,
    text_font: TextInputTextFont,
    text_color: TextInputTextColor,
    text_input: TextInput,
    local_setting: StorageLink,
}

impl TextInputBundle {
    pub fn new(node: Node, text_font: TextFont, setting: Entity) -> Self {
        Self {
            node,
            text_font: TextInputTextFont(text_font),
            text_color: TextInputTextColor(SECONDARY_COLOR.into()),
            text_input: TextInput,
            local_setting: StorageLink::new(setting),
        }
    }
}

#[derive(Debug, Bundle)]
pub struct SettingOptionButtonBundle<T: Component> {
    node: Node,
    visibility: Visibility,
    background_color: BackgroundColor,
    button: Button,
    setting_option: SettingOption,
    local_setting: StorageLink,
    value: T,
}

impl<T: Component> SettingOptionButtonBundle<T> {
    pub fn new(value: T, setting: Entity, border_size: Val, visible: bool) -> Self {
        let mut node = common::menu_item_node();
        node.border = UiRect::all(border_size);
        let visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        Self {
            node,
            visibility,
            background_color: PRIMARY_COLOR.into(),
            button: Button,
            setting_option: SettingOption,
            local_setting: StorageLink::new(setting),
            value,
        }
    }
}

#[derive(Bundle)]
pub struct ImageBundle {
    node: Node,
    image: ImageNode,
}

impl ImageBundle {
    pub fn new(node: Node, image: Handle<Image>) -> Self {
        Self {
            node,
            image: ImageNode::new(image),
        }
    }
}

#[derive(Bundle)]
pub struct TextBundle {
    node: Node,
    text: Text,
    text_layout: TextLayout,
    text_font: TextFont,
    text_color: TextColor,
}

impl TextBundle {
    pub fn new(text: impl Into<String>, text_font: TextFont) -> Self {
        Self {
            node: Default::default(),
            text: Text::new(text),
            text_layout: TextLayout::new_with_justify(JustifyText::Center),
            text_font,
            text_color: TextColor(SECONDARY_COLOR.into()),
        }
    }
}
