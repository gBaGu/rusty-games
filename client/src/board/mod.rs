mod components;
mod events;
mod systems;

use bevy::prelude::{App, Plugin, Update};

pub use components::BoardBundle;
pub use events::{ButtonContentArrived, ButtonPressed};
use systems::{add_content, button_press, create};

pub struct BoardPlugin;

impl Plugin for BoardPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ButtonPressed>()
            .add_event::<ButtonContentArrived>()
            .add_systems(Update, (create, button_press, add_content));
    }
}
