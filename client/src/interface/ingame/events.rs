use bevy::prelude::*;

#[derive(Event)]
pub struct PlayerInfoReady {
    pub id: u64,
    pub image: Handle<Image>,
}
