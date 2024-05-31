use bevy::prelude::{default, Bundle, ButtonBundle, Component, Entity, Resource, Style, UiImage};

#[derive(Debug, Default, Resource)]
pub struct Settings {
    user_id: Option<u64>,
}

impl Settings {
    pub fn user_id(&self) -> Option<u64> {
        self.user_id
    }

    pub fn set_user_id(&mut self, value: u64) {
        self.user_id = Some(value);
    }
}

#[derive(Debug, Component)]
pub struct SubmitTextInputSetting<T> {
    associated_input: Entity,
    setter: fn(&mut Settings, T),
}

impl<T> SubmitTextInputSetting<T> {
    pub fn new(entity: Entity, setter: fn(&mut Settings, T)) -> Self {
        Self {
            associated_input: entity,
            setter,
        }
    }

    pub fn associated_input(&self) -> Entity {
        self.associated_input
    }

    pub fn submit(&self, settings: &mut Settings, value: T) {
        (self.setter)(settings, value);
    }
}

pub fn submit_text_input_setting<T: 'static>(
    style: Style,
    input_id: Entity,
    setter: fn(&mut Settings, T),
) -> impl Bundle {
    (
        ButtonBundle {
            style,
            image: UiImage::default(),
            ..default()
        },
        SubmitTextInputSetting::new(input_id, setter),
    )
}
