use std::marker::PhantomData;

use bevy::prelude::*;
use bevy::tasks::Task;
use game_server::rpc_server::rpc::RpcResult;
use tonic::transport;

use super::{GameSessionUpdate, GrpcResult, GAME_SESSION_RECONNECT_INTERVAL_SEC};

/// Task component for connecting to grpc server
#[derive(Component, Deref, DerefMut)]
pub struct ConnectClientTask(pub Task<Result<transport::Channel, transport::Error>>);

/// Component for grpc call task
#[derive(Component, Deref, DerefMut)]
pub struct CallTask<T>(Task<RpcResult<T>>);

impl<T> CallTask<T> {
    pub fn new(task: Task<RpcResult<T>>) -> Self {
        Self(task)
    }
}

/// Task component for receiving grpc connection status
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveConnectionStatusTask(pub Task<Result<bool, async_channel::RecvError>>);

/// Task component for sending actions of type `T` into the game session input channel.
#[derive(Component, Deref, DerefMut)]
pub struct SendActionTask<T>(pub Task<Result<(), async_channel::SendError<T>>>);

/// Task component for receiving actions of type `T` from the game session output channel.
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveSessionUpdateTask<T>(
    pub Task<Result<GrpcResult<GameSessionUpdate<T>>, async_channel::RecvError>>,
);

/// Component that contains task with `GameSession` streaming call and
/// channels to send/receive action data of type `T` to/from the server.
#[derive(Debug, Component)]
pub struct GameSession<G, T> {
    task: Task<()>,
    action_sender: async_channel::Sender<T>,
    update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    _phantom_data: PhantomData<G>,
}

impl<G, T> GameSession<G, T> {
    pub fn new(
        task: Task<()>,
        action_sender: async_channel::Sender<T>,
        update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    ) -> Self {
        Self {
            task,
            action_sender,
            update_receiver,
            _phantom_data: Default::default(),
        }
    }

    pub fn action_sender(&self) -> async_channel::Sender<T> {
        self.action_sender.clone()
    }

    pub fn update_receiver(&self) -> async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>> {
        self.update_receiver.clone()
    }

    pub fn task_mut(&mut self) -> &mut Task<()> {
        &mut self.task
    }
}

/// Component that indicates that reconnect process is started.
/// `T` is the type of the game (required for grpc requests).
#[derive(Debug, Component)]
pub struct ReconnectSession<T>(PhantomData<T>);

impl<T> Default for ReconnectSession<T> {
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
    tag: ReconnectSession<T>,
}

impl<T: Send + Sync + 'static> Default for ReconnectSessionBundle<T> {
    fn default() -> Self {
        Self {
            timer: Default::default(),
            tag: Default::default(),
        }
    }
}
