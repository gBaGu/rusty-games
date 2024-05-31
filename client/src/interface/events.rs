use bevy::prelude::{Entity, Event};

#[derive(Debug, Event)]
pub struct SubmitPressed {
    pub source: Entity,
}
