use bevy::prelude::Resource;

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
