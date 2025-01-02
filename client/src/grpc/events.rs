use bevy::prelude::*;
use game_server::rpc_server::rpc::RpcResult;

use super::{GameSessionUpdate, GrpcResult};

#[derive(Event)]
pub struct Connected;

#[derive(Event)]
pub struct Disconnected;

#[derive(Debug, Event)]
pub struct RpcResultReady<T> {
    entity: Entity,
    result: RpcResult<T>,
}

impl<T> RpcResultReady<T> {
    pub fn new(entity: Entity, result: RpcResult<T>) -> Self {
        Self {
            entity,
            result,
        }
    }

    pub fn entity(&self) -> Entity {
        self.entity
    }

    pub fn result(&self) -> &RpcResult<T> {
        &self.result
    }
}

/// Event that is required to initialize game session opening process.
/// When `delayed` is set to `true` opening will be delayed.
#[derive(Debug, Event)]
pub struct OpenSession {
    game: Entity,
    delayed: bool,
}

impl OpenSession {
    pub fn new(game: Entity) -> Self {
        Self {
            game,
            delayed: false,
        }
    }

    pub fn new_delayed(game: Entity) -> Self {
        Self {
            game,
            delayed: true,
        }
    }

    pub fn game(&self) -> Entity {
        self.game
    }

    pub fn delayed(&self) -> bool {
        self.delayed
    }
}

/// Event that is required to trigger game session to be closed.
/// Contains an [`Entity`] of a game session.
#[derive(Debug, Deref, Event)]
pub struct CloseSession(Entity);

impl CloseSession {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Event that indicates that session initialization is finished.
/// Contains an [`Entity`] of a game session.
#[derive(Debug, Deref, Event)]
pub struct SessionOpened(Entity);

impl SessionOpened {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Event that indicates that game session task is finished.
/// Contains an [`Entity`] of a game session.
#[derive(Debug, Deref, Event)]
pub struct SessionFinished(Entity);

impl SessionFinished {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

/// Event that indicates that an error occurred during action send attempt.  
/// Contains an [`Entity`] of a game session.
#[derive(Copy, Clone, Debug, Deref, Event)]
pub struct SendSessionActionFailed(Entity);

impl SendSessionActionFailed {
    pub fn new(entity: Entity) -> Self {
        Self(entity)
    }
}

#[derive(Debug, Event)]
pub struct SendSessionAction<T> { // TODO: rename OutgoingSessionActionReady
    session_entity: Entity,
    action: T,
}

impl<T> SendSessionAction<T> {
    pub fn new(entity: Entity, action: T) -> Self {
        Self {
            session_entity: entity,
            action,
        }
    }

    pub fn session_entity(&self) -> Entity {
        self.session_entity
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

#[derive(Debug, Event)]
pub struct SessionUpdateReceived<T> {
    session_entity: Entity,
    update: GrpcResult<GameSessionUpdate<T>>,
}

impl<T> SessionUpdateReceived<T> {
    pub fn new(entity: Entity, update: GrpcResult<GameSessionUpdate<T>>) -> Self {
        Self {
            session_entity: entity,
            update,
        }
    }

    pub fn session_entity(&self) -> Entity {
        self.session_entity
    }

    pub fn update(&self) -> &GrpcResult<GameSessionUpdate<T>> {
        &self.update
    }
}
