use bevy::asset::AssetServer;
use bevy::ecs::bundle::Bundle;
use bevy::ecs::change_detection::Res;
use bevy::render::color::Color;
use bevy::text::TextStyle;
use bevy::ui::node_bundles::{ButtonBundle, NodeBundle, TextBundle};
use bevy::ui::{AlignItems, FlexDirection, JustifyContent, Style, UiImage, UiRect, Val};
use bevy::utils::default;
use bevy_simple_text_input::{TextInputBundle, TextInputTextStyle};

use crate::app_state::{AppState, AppStateTransition};

const FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";

const MENU_ITEM_HEIGHT: f32 = 50.0;
const MENU_ITEM_WIDTH: f32 = 300.0;
const MENU_FONT_SIZE: f32 = 40.0;
const MENU_TEXT_COLOR: Color = Color::OLIVE;

// Styles

pub fn menu_item_style() -> Style {
    Style {
        width: Val::Px(MENU_ITEM_WIDTH),
        height: Val::Px(MENU_ITEM_HEIGHT),
        margin: UiRect::all(Val::Px(10.0)),
        justify_content: JustifyContent::Center,
        align_items: AlignItems::Center,
        ..default()
    }
}

pub fn menu_text_style(asset_server: Res<AssetServer>) -> TextStyle {
    TextStyle {
        font: asset_server.load(FONT_PATH),
        font_size: MENU_FONT_SIZE,
        color: MENU_TEXT_COLOR,
    }
}

// Containers

pub fn global_column_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            height: Val::Percent(100.0),
            width: Val::Percent(100.0),
            ..default()
        },
        ..default()
    }
}

pub fn menu_column_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    }
}

pub fn menu_row_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    }
}

// Text

pub fn menu_text_bundle(text: &str, text_style: TextStyle) -> TextBundle {
    TextBundle::from_section(text, text_style)
}

pub fn menu_text_input_bundle(text_style: TextStyle, style: Style) -> impl Bundle {
    (
        TextInputBundle {
            text_style: TextInputTextStyle(text_style),
            ..default()
        },
        NodeBundle { style, ..default() },
    )
}

// Buttons

pub mod button_bundle {
    use super::*;
    use crate::interface::components::AssociatedTextInput;
    use crate::settings::{Settings, SubmitTextInputSetting};
    use bevy::prelude::Entity;

    pub fn exit(style: Style) -> impl Bundle {
        (
            ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            AppStateTransition(None),
        )
    }

    pub fn menu_navigation(style: Style, new_state: AppState) -> impl Bundle {
        (
            ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            AppStateTransition(Some(new_state)),
        )
    }

    pub fn menu_navigation_with_associated_text_input(
        style: Style,
        new_state: AppState,
        input_id: Entity,
    ) -> impl Bundle {
        (
            ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            AppStateTransition(Some(new_state)),
            AssociatedTextInput(input_id),
        )
    }

    pub fn submit_text_input_setting<T: 'static>(
        style: Style,
        input_id: Entity,
        setter: fn(&mut Settings, T),
    ) -> impl Bundle {
        (
            ButtonBundle {
                style,
                image: UiImage::default(),
                ..default()
            },
            SubmitTextInputSetting::new(input_id, setter),
        )
    }
}
