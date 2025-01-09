use std::ops::DerefMut;

use bevy::ecs::component::Component;
use bevy::ecs::system::EntityCommands;
use bevy::tasks;
use bevy::tasks::futures_lite::future;

pub enum TaskCleaningStrategy {
    RemoveComponent,
    Despawn,
}

/// Used to clean up finished task components.
/// [`Self::STRATEGY`] is used to control whether to remove this component from the entity or
/// despawn it completely.
pub trait PollOnce: DerefMut<Target = tasks::Task<Self::Output>> + Component + Sized {
    /// A type returned from a completed task.
    type Output;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::Despawn;

    /// Calls [`future::poll_once`] on a task and if it's completed cleanup entity
    /// according to [`Self::STRATEGY`].
    /// [`Entity`] from `cmds` must be the same as the one containing `self`.
    fn poll_once(&mut self, mut cmds: EntityCommands) -> Option<Self::Output> {
        tasks::block_on(future::poll_once(self.deref_mut())).inspect(|_| match Self::STRATEGY {
            TaskCleaningStrategy::RemoveComponent => {
                cmds.remove::<Self>();
            }
            TaskCleaningStrategy::Despawn => cmds.despawn(),
        })
    }
}
