mod components;
mod events;
mod systems;

use bevy::prelude::{App, Plugin, Update};

pub use components::BoardBundle;
pub use events::{TileFilled, TilePressed};
use systems::{set_tile_image, create, handle_input};

pub const BORDER_WIDTH: f32 = 1.0;

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<TilePressed>()
            .add_event::<TileFilled>()
            .add_systems(Update, (create, handle_input, set_tile_image));
    }
}
