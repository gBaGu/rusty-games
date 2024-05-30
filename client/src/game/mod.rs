mod error;
mod resources;
mod game_info;
mod systems;
mod events;
mod components;

use bevy::prelude::{App, Plugin, Update};

pub use components::Position;
pub use events::{CellUpdated, GameOver, MockBotGame, StateUpdated};
pub use game_info::{FullGameInfo, GameInfo};
pub use resources::CurrentGame;

pub const GAME_REFRESH_INTERVAL_SEC: f32 = 1.0;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<StateUpdated>()
            .add_event::<CellUpdated>()
            .add_event::<GameOver>()
            .add_event::<MockBotGame>();
    }
}
