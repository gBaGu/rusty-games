use bevy::prelude::*;
use game_server::core;
use game_server::rpc_server::RpcResult;

use super::error::GrpcError;
use crate::util;

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

util::entity_type!(
    /// Event that is required to trigger game session to be closed.
    /// Contains an [`Entity`] of a game session.
    CloseSession, Event
);

util::entity_type!(
    /// Event that indicates that session initialization is finished.
    /// Contains an [`Entity`] of a game session.
    SessionOpened, Event
);

util::entity_type!(
    /// Event that indicates that game session task is finished.
    /// Contains an [`Entity`] of a game session.
    SessionClosed, Event
);

/// Event that indicates that an error occurred during action send attempt.  
/// Contains an [`Entity`] of a game session and an action of type `T` that is failed to send.
#[derive(Debug, Event)]
pub struct SessionActionSendFailed<T> {
    session_entity: Entity,
    action: T,
}

impl<T> SessionActionSendFailed<T> {
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

/// Event that indicates that an action is ready to be sent to the server over the game session.
/// Contains an [`Entity`] that has the game session component and an action that needs to be sent.
#[derive(Debug, Event)]
pub struct SessionActionReadyToSend<T> {
    session_entity: Entity,
    action: T,
}

impl<T> SessionActionReadyToSend<T> {
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

/// Event that indicates that an action was receiver from the server.
#[derive(Debug, Event)]
pub struct SessionUpdateReceived<T> {
    session_entity: Entity,
    player: core::PlayerPosition,
    action: T,
}

impl<T> SessionUpdateReceived<T> {
    pub fn new(entity: Entity, player: core::PlayerPosition, action: T) -> Self {
        Self {
            session_entity: entity,
            player,
            action,
        }
    }

    pub fn session_entity(&self) -> Entity {
        self.session_entity
    }

    pub fn player(&self) -> core::PlayerPosition {
        self.player
    }

    pub fn action(&self) -> &T {
        &self.action
    }
}

/// Event that indicates that an error was receiver from the server.
#[derive(Debug, Event)]
pub struct SessionErrorReceived {
    session_entity: Entity,
    error: GrpcError,
}

impl SessionErrorReceived {
    pub fn new(entity: Entity, error: GrpcError) -> Self {
        Self {
            session_entity: entity,
            error,
        }
    }

    pub fn session_entity(&self) -> Entity {
        self.session_entity
    }

    pub fn error(&self) -> &GrpcError {
        &self.error
    }
}

#[derive(Debug, Deref, Event)]
pub struct AuthLinkReceived(String);

impl AuthLinkReceived {
    pub fn new(link: String) -> Self {
        Self(link)
    }
}

#[derive(Debug, Deref, Event)]
pub struct AuthTokenReceived(String);

impl AuthTokenReceived {
    pub fn new(link: String) -> Self {
        Self(link)
    }
}

/// Event that contains the id of a user that has just logged in.
#[derive(Debug, Deref, Event)]
pub struct LogInSuccess(u64);

impl LogInSuccess {
    pub fn new(user_id: u64) -> Self {
        Self(user_id)
    }
}

#[derive(Debug, Deref, Event)]
pub struct LogInFailed(GrpcError);

impl LogInFailed {

    pub fn new(error: GrpcError) -> Self {
        Self(error)
    }
}

/// An event that is used to trigger jwt token drop.
#[derive(Debug, Event)]
pub struct LogOut;
