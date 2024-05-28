use bevy::prelude::*;

use crate::board::components::Position;

/// Event emitted when board button is pressed.
/// Contains entity of a board whose button was pressed
/// and a [`Position`] of a button.
#[derive(Debug, Event)]
pub struct ButtonPressed {
    pub board: Entity,
    pub pos: Position,
}

/// Event used by this plugin to update image inside a button.
/// Contains entity of a board whose button needs to be updated,
/// button [`Position`] and an image.
#[derive(Clone, Debug, Event)]
pub struct ButtonContentArrived {
    pub board: Entity,
    pub pos: Position,
    pub image: Handle<Image>,
}
