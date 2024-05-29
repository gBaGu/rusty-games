mod components;
mod events;
mod systems;

use bevy::prelude::*;

pub use components::InGameUI;
pub use events::PlayerInfoReady;
use systems::{create, handle_state_update, update_player_info};

pub const FONT_PATH: &str = "fonts/FiraSans-Bold.ttf";
pub const FONT_SIZE: f32 = 40.0;
pub const ITEM_HEIGHT: f32 = 50.0;
pub const TEXT_COLOR: Color = Color::OLIVE;
pub const PLAYER_TURN_COLOR: Color = Color::DARK_GREEN;
pub const ENEMY_TURN_COLOR: Color = Color::RED;

pub struct InGameUIPlugin;

impl Plugin for InGameUIPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerInfoReady>().add_systems(
            Update,
            (
                create,
                handle_state_update.after(create),
                update_player_info.after(create),
            ),
        );
    }
}
