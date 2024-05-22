use bevy::hierarchy::{BuildChildren, ChildBuilder};
use bevy::prelude::{default, Bundle, ButtonBundle, TextBundle, UiImage, Component, Deref};
use bevy::text::TextStyle;
use bevy::ui::Style;

use crate::app_state::{AppState, AppStateTransition};
use crate::game::GameInfo;

#[derive(Clone, Debug, Deref, Component)]
pub struct JoinGame(pub GameInfo);

#[derive(Bundle)]
pub struct MenuNavigationButtonBundle {
    button: ButtonBundle,
    state_transition: AppStateTransition,
}

#[derive(Bundle)]
pub struct JoinGameButtonBundle {
    button: ButtonBundle,
    join: JoinGame,
}

pub fn spawn_exit_button(
    builder: &mut ChildBuilder,
    style: Style,
    text_style: TextStyle,
    text: impl Into<String>,
) {
    builder
        .spawn(MenuNavigationButtonBundle {
            button: ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            state_transition: AppStateTransition(None),
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(text, text_style));
        });
}

pub fn spawn_menu_navigation_button(
    builder: &mut ChildBuilder,
    style: Style,
    text_style: TextStyle,
    text: impl Into<String>,
    new_state: AppState,
) {
    builder
        .spawn(MenuNavigationButtonBundle {
            button: ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            state_transition: AppStateTransition(Some(new_state)),
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(text, text_style));
        });
}

pub fn spawn_join_game_button_bundle(
    builder: &mut ChildBuilder,
    style: Style,
    text_style: TextStyle,
    text: impl Into<String>,
    game: GameInfo,
) {
    builder
        .spawn(JoinGameButtonBundle {
            button: ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            join: JoinGame(game),
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(text, text_style));
        });
}
