use bevy::ecs::{component::Component, entity::Entity};

#[derive(Debug, Component)]
pub struct AssociatedTextInput(pub Entity);

#[derive(Debug, Component)]
pub struct NextPlayerImage;
