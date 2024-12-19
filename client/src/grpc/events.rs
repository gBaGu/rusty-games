use bevy::prelude::*;
use game_server::rpc_server::rpc::RpcResult;

use super::{GameSessionUpdate, GrpcResult};

#[derive(Event)]
pub struct Connected;

#[derive(Event)]
pub struct Disconnected;

#[derive(Deref, Event)]
pub struct RpcResultReady<T>(RpcResult<T>);

impl<T> RpcResultReady<T> {
    pub fn new(val: RpcResult<T>) -> Self {
        Self(val)
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
