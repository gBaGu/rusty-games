use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::tasks::Task;
use game_server::rpc_server::RpcResult;
use tonic::transport;

use super::{GameSessionUpdate, GrpcResult, GAME_SESSION_RECONNECT_INTERVAL_SEC};
use crate::common::{PollOnce, TaskCleaningStrategy};

/// Task component for connecting to grpc server
#[derive(Component, Deref, DerefMut)]
pub struct ConnectClientTask(Task<Result<transport::Channel, transport::Error>>);

impl From<Task<Result<transport::Channel, transport::Error>>> for ConnectClientTask {
    fn from(value: Task<Result<transport::Channel, transport::Error>>) -> Self {
        Self(value)
    }
}

impl PollOnce for ConnectClientTask {
    type Output = Result<transport::Channel, transport::Error>;
}

/// Component for grpc call task
#[derive(Component, Deref, DerefMut)]
pub struct CallTask<T>(Task<RpcResult<T>>);

impl<T> From<Task<RpcResult<T>>> for CallTask<T> {
    fn from(value: Task<RpcResult<T>>) -> Self {
        Self(value)
    }
}

impl<T: Send + 'static> PollOnce for CallTask<T> {
    type Output = RpcResult<T>;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::RemoveComponent;
}

/// Task component for receiving grpc connection status
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveConnectionStatusTask(Task<Result<bool, async_channel::RecvError>>);

impl From<Task<Result<bool, async_channel::RecvError>>> for ReceiveConnectionStatusTask {
    fn from(value: Task<Result<bool, async_channel::RecvError>>) -> Self {
        Self(value)
    }
}

impl PollOnce for ReceiveConnectionStatusTask {
    type Output = Result<bool, async_channel::RecvError>;
}

/// Task component for sending encoded actions into the game session input channel.
#[derive(Component, Deref, DerefMut)]
pub struct SendActionTask(Task<Result<(), async_channel::SendError<Vec<u8>>>>);

impl From<Task<Result<(), async_channel::SendError<Vec<u8>>>>> for SendActionTask {
    fn from(value: Task<Result<(), async_channel::SendError<Vec<u8>>>>) -> Self {
        Self(value)
    }
}

impl PollOnce for SendActionTask {
    type Output = Result<(), async_channel::SendError<Vec<u8>>>;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::RemoveComponent;
}

/// Task component for receiving actions of type `T` from the game session output channel.
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveSessionUpdateTask<T>(Task<GrpcResult<GameSessionUpdate<T>>>);

impl<T> From<Task<GrpcResult<GameSessionUpdate<T>>>> for ReceiveSessionUpdateTask<T> {
    fn from(value: Task<GrpcResult<GameSessionUpdate<T>>>) -> Self {
        Self(value)
    }
}

impl<T: Send + 'static> PollOnce for ReceiveSessionUpdateTask<T> {
    type Output = GrpcResult<GameSessionUpdate<T>>;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::RemoveComponent;
}

/// Task component for receiving log in link from `LogIn` grpc request.
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveLogInLinkTask(Task<GrpcResult<String>>);

impl From<Task<GrpcResult<String>>> for ReceiveLogInLinkTask {
    fn from(value: Task<GrpcResult<String>>) -> Self {
        Self(value)
    }
}

impl PollOnce for ReceiveLogInLinkTask {
    type Output = GrpcResult<String>;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::RemoveComponent;
}

/// Task component for receiving jwt token from `LogIn` grpc request.
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveLogInTokenTask(Task<GrpcResult<String>>);

impl From<Task<GrpcResult<String>>> for ReceiveLogInTokenTask {
    fn from(value: Task<GrpcResult<String>>) -> Self {
        Self(value)
    }
}

impl PollOnce for ReceiveLogInTokenTask {
    type Output = GrpcResult<String>;
    const STRATEGY: TaskCleaningStrategy = TaskCleaningStrategy::RemoveComponent;
}

/// Component to indicate that the entity is a `LogIn` request.
/// In order to make `LogIn` call this component needs to be spawned.
#[derive(Component)]
pub struct LogInRequest;

/// Component that contains task with `LogIn` server-streaming call and
/// channels to receive log in link and token from the server.
#[derive(Debug, Component)]
pub struct LogInTask {
    task: Task<GrpcResult<()>>,
    link_receiver: async_channel::Receiver<String>,
    token_receiver: async_channel::Receiver<String>,
}

impl LogInTask {
    pub fn new(
        task: Task<GrpcResult<()>>,
        link_receiver: async_channel::Receiver<String>,
        token_receiver: async_channel::Receiver<String>,
    ) -> Self {
        Self {
            task,
            link_receiver,
            token_receiver,
        }
    }

    pub fn link_receiver(&self) -> async_channel::Receiver<String> {
        self.link_receiver.clone()
    }

    pub fn token_receiver(&self) -> async_channel::Receiver<String> {
        self.token_receiver.clone()
    }

    pub fn task_mut(&mut self) -> &mut Task<GrpcResult<()>> {
        &mut self.task
    }
}

/// Component that contains task with `GameSession` streaming call and
/// channels to send/receive action data of type `T` to/from the server.
#[derive(Debug, Component)]
pub struct GameSession<G, T> {
    task: Task<()>,
    action_sender: async_channel::Sender<Vec<u8>>,
    update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    _phantom_data: PhantomData<G>,
}

impl<G, T> GameSession<G, T> {
    pub fn new(
        task: Task<()>,
        action_sender: async_channel::Sender<Vec<u8>>,
        update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    ) -> Self {
        Self {
            task,
            action_sender,
            update_receiver,
            _phantom_data: Default::default(),
        }
    }

    pub fn action_sender(&self) -> async_channel::Sender<Vec<u8>> {
        self.action_sender.clone()
    }

    pub fn update_receiver(&self) -> async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>> {
        self.update_receiver.clone()
    }

    pub fn task_mut(&mut self) -> &mut Task<()> {
        &mut self.task
    }
}

/// Component that indicates that game session connect process is started.
/// `T` is the type of the game (required for grpc requests).
#[derive(Debug, Component)]
pub struct ConnectingGameSession<T>(PhantomData<T>);

impl<T> Default for ConnectingGameSession<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

/// Timer component that counts the time before game session will be reopened.  
/// The [`Timer`] is set to [`GAME_SESSION_RECONNECT_INTERVAL_SEC`].
#[derive(Debug, Component, Deref, DerefMut)]
pub struct ReconnectSessionTimer(Timer);

impl Default for ReconnectSessionTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(
            GAME_SESSION_RECONNECT_INTERVAL_SEC,
            TimerMode::Once,
        ))
    }
}

#[derive(Debug, Bundle)]
pub struct ReconnectSessionBundle<T: Send + Sync + 'static> {
    pub timer: ReconnectSessionTimer,
    tag: ConnectingGameSession<T>,
}

impl<T: Send + Sync + 'static> Default for ReconnectSessionBundle<T> {
    fn default() -> Self {
        Self {
            timer: Default::default(),
            tag: Default::default(),
        }
    }
}
