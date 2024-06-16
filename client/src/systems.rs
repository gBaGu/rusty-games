use bevy::prelude::*;
use bevy::window::WindowResized;

use super::Background;

pub const BACKGROUND_COLOR: Color = Color::rgb(0.38, 0.5, 0.38);

pub fn init_app(mut commands: Commands, window: Query<&Window>) {
    commands.spawn(Camera2dBundle::default());
    let window = window.single();
    commands.spawn((
        SpriteBundle {
            sprite: Sprite {
                color: BACKGROUND_COLOR,
                custom_size: Some(Vec2::new(window.width(), window.height())),
                ..Default::default()
            },
            ..default()
        },
        Background,
    ));
}

pub fn on_resize(
    mut q: Query<&mut Sprite, With<Background>>,
    mut resize_reader: EventReader<WindowResized>,
) {
    let mut sprite = q.single_mut();
    for e in resize_reader.read() {
        sprite.custom_size = Some(Vec2::new(e.width, e.height));
    }
}