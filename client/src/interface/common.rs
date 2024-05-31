use bevy::asset::AssetServer;
use bevy::render::color::Color;
use bevy::text::TextStyle;
use bevy::ui::node_bundles::NodeBundle;
use bevy::ui::{AlignItems, FlexDirection, JustifyContent, Style, UiRect, Val};
use bevy::utils::default;

pub const CONFIRMATION_SOUND_PATH: &str = "audio/confirmation.ogg";
pub const ERROR_SOUND_PATH: &str = "audio/error.ogg";
pub const TURN_SOUND_PATH: &str = "audio/turn.ogg";

pub const FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";

pub const MENU_ITEM_HEIGHT: f32 = 50.0;
pub const MENU_ITEM_WIDTH: f32 = 300.0;
pub const MENU_LIST_MIN_HEIGHT: f32 = MENU_ITEM_HEIGHT * 6.0;
pub const MENU_FONT_SIZE: f32 = 40.0;
pub const MENU_TEXT_COLOR: Color = Color::OLIVE;
pub const GAME_LIST_REFRESH_INTERVAL_SEC: f32 = 5.0;

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

pub fn menu_text_style(asset_server: &AssetServer) -> TextStyle {
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

pub fn column_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            min_height: Val::Px(MENU_LIST_MIN_HEIGHT),
            ..default()
        },
        ..default()
    }
}

pub fn row_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Center,
            ..default()
        },
        ..default()
    }
}
