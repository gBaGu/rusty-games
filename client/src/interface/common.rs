use bevy::prelude::*;

pub const CONFIRMATION_SOUND_PATH: &str = "audio/confirmation.ogg";
pub const ERROR_SOUND_PATH: &str = "audio/error.ogg";
pub const TURN_SOUND_PATH: &str = "audio/turn.ogg";

pub const FONT_PATH: &str = "fonts/ADLaMDisplay-Regular.ttf";

pub const LOGO_HEIGHT: f32 = 200.0;
pub const LOGO_WIDTH: f32 = 200.0;
pub const MENU_ITEM_HEIGHT: f32 = 50.0;
pub const MENU_ITEM_WIDTH: f32 = 300.0;
pub const MENU_LIST_MIN_HEIGHT: f32 = MENU_ITEM_HEIGHT * 6.0;
pub const FONT_SIZE: f32 = 30.0;
pub const GAME_LIST_REFRESH_INTERVAL_SEC: f32 = 5.0;

pub const OVERLAY_BACKGROUND_COLOR: Color = Color::srgba(0.25, 0.25, 0.25, 0.95);
pub const PRIMARY_COLOR: Color = Color::srgb(0.29, 0.40, 0.29);
pub const SECONDARY_COLOR: Color = Color::srgb(0.88, 1.0, 0.88);

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
        font_size: FONT_SIZE,
        color: SECONDARY_COLOR,
    }
}

// Containers

pub fn root_node_bundle() -> NodeBundle {
    NodeBundle {
        style: Style {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
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
