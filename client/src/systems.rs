use bevy::prelude::*;
use bevy::window::WindowResized;

use super::{Background, Settings, UserIdChanged};
use crate::grpc::LogInSuccess;

pub const BACKGROUND_COLOR: Color = Color::srgb(0.38, 0.5, 0.38);

pub fn init_app(mut commands: Commands, window: Query<&Window>) {
    commands.spawn(Camera2d::default());
    let window = window.single();
    commands.spawn((
        Sprite {
            color: BACKGROUND_COLOR,
            custom_size: Some(Vec2::new(window.width(), window.height())),
            ..Default::default()
        },
        Transform::from_translation(Vec2::splat(0.0).extend(-1.0)),
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

/// Listen to [`LogInSuccess`] event, update user id in settings and send [`UserIdChanged`] event.
pub fn update_user_id(
    mut log_in_success: EventReader<LogInSuccess>,
    mut user_id_changed: EventWriter<UserIdChanged>,
    mut settings: ResMut<Settings>,
) {
    for event in log_in_success.read() {
        let user_id = **event;
        info!("new user id: {}", user_id);
        settings.set_user_id(user_id);
        user_id_changed.send(UserIdChanged::new(user_id));
    }
}
