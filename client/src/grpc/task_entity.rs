use std::future::Future;
use std::ops::DerefMut;

use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future};

/// This struct is intended for use with entities that only needed to wait on a task.
/// It makes sure that task entity is despawned after successful poll
pub struct TaskEntity<'a, 'w, 's, F> {
    commands: Commands<'w, 's>,
    entity: Entity,
    task: &'a mut F,
}

impl<'a, 'w, 's, F> TaskEntity<'a, 'w, 's, F> {
    pub fn new(commands: Commands<'w, 's>, entity: Entity, task: &'a mut F) -> Self {
        Self {
            commands,
            entity,
            task,
        }
    }
}

impl<'a, 'w, 's, F, T> TaskEntity<'_, '_, '_, F>
where
    F: Future<Output = T> + Unpin,
{
    /// Polls future and in case if result is ready adds despawn command to `commands`
    pub fn poll_once(&mut self) -> Option<T> {
        block_on(future::poll_once(self.task.deref_mut())).and_then(|res| {
            self.commands.entity(self.entity).despawn();
            Some(res)
        })
    }
}