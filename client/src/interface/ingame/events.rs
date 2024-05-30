use bevy::prelude::*;

#[derive(Debug, Event)]
pub struct PlayerInfoReady {
    pub id: u64,
    pub image: Handle<Image>,
}

impl PlayerInfoReady {
    pub fn new(id: u64, image: Handle<Image>) -> Self {
        Self { id, image }
    }
}
