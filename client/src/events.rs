use bevy::prelude::*;

/// Event that is fired after user id is changed.
#[derive(Debug, Event)]
pub struct UserIdChanged {
    new_user_id: u64,
}

impl UserIdChanged {
    pub fn new(user_id: u64) -> Self {
        Self {
            new_user_id: user_id,
        }
    }

    pub fn new_user_id(&self) -> u64 {
        self.new_user_id
    }
}