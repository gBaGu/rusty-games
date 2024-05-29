use bevy::prelude::*;

#[derive(Debug, Event)]
pub struct PlayerInfoReady {
    pub id: u64,
    pub image: Handle<Image>,
}
