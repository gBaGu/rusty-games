use std::ops::DerefMut;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, Task};

/// This struct is intended for use with entities that only needed to wait on a task.
/// It makes sure that task entity is despawned after successful poll
pub struct TaskEntity<'a, 'w, 's, T> {
    commands: Commands<'w, 's>,
    entity: Entity,
    task: &'a mut Task<T>,
}

impl<'a, 'w, 's, T> TaskEntity<'a, 'w, 's, T> {
    pub fn new(commands: Commands<'w, 's>, entity: Entity, task: &'a mut Task<T>) -> Self {
        Self {
            commands,
            entity,
            task,
        }
    }

    /// Polls future and in case if result is ready adds despawn command to `commands`
    pub fn poll_once(&mut self) -> Option<T> {
        block_on(future::poll_once(self.task.deref_mut())).and_then(|res| {
            self.commands.entity(self.entity).despawn();
            Some(res)
        })
    }
}
