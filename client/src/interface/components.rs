use bevy::ecs::{component::Component, entity::Entity};

#[derive(Debug, Component)]
pub struct AssociatedGameList(pub Entity);

#[derive(Debug, Component)]
pub struct AssociatedTextInput(pub Entity);
