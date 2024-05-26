use bevy::ecs::{component::Component, entity::Entity};
use bevy::prelude::{default, Bundle, NodeBundle, Style, UiRect, Val};

#[derive(Debug, Component)]
pub struct AssociatedTextInput(pub Entity);

#[derive(Debug, Component)]
pub struct NextPlayerImage;

#[derive(Bundle, Debug)]
pub struct EmptyNextPlayerImageBundle {
    node_bundle: NodeBundle,
    next_player_image: NextPlayerImage,
}

pub fn empty_next_player_image(size: Val) -> EmptyNextPlayerImageBundle {
    EmptyNextPlayerImageBundle {
        node_bundle: NodeBundle {
            style: Style {
                width: size,
                height: size,
                margin: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            ..default()
        },
        next_player_image: NextPlayerImage,
    }
}
