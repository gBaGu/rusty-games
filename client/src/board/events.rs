use bevy::prelude::*;

use crate::board::components::Position;

#[derive(Debug, Event)]
pub struct ButtonPressed {
    pub board: Entity,
    pub pos: Position,
}

#[derive(Clone, Debug, Event)]
pub struct ButtonContentArrived {
    pub board: Entity,
    pub pos: Position,
    pub image: Handle<Image>,
}
