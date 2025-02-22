use bevy::prelude::Resource;

#[derive(Debug, Default, Resource)]
pub struct Settings {
    user_id: Option<u64>,
}

impl Settings {
    pub fn builder() -> SettingsBuilder {
        SettingsBuilder::default()
    }

    pub fn user_id(&self) -> Option<u64> {
        self.user_id
    }

    pub fn set_user_id(&mut self, value: u64) {
        self.user_id = Some(value);
    }

    pub fn reset_user_id(&mut self) {
        self.user_id = None;
    }
}

#[derive(Default)]
pub struct SettingsBuilder {
    user_id: Option<u64>,
}

impl SettingsBuilder {
    pub fn build(&self) -> Settings {
        Settings {
            user_id: self.user_id,
        }
    }

    pub fn user_id(&mut self, user_id: u64) -> &mut Self {
        let _ = self.user_id.insert(user_id);
        self
    }
}
