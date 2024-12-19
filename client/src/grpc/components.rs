use bevy::prelude::*;
use bevy::tasks;
use bevy::tasks::futures_lite::future;
use bevy::tasks::Task;
use game_server::rpc_server::rpc::RpcResult;
use tonic::transport;

use super::{GameSessionUpdate, GrpcResult};

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

#[derive(Component, Deref, DerefMut)]
pub struct SendActionTask<T>(pub Task<Result<(), async_channel::SendError<T>>>);

#[derive(Component, Deref, DerefMut)]
pub struct ReceiveSessionUpdateTask<T>(
    pub Task<Result<GrpcResult<GameSessionUpdate<T>>, async_channel::RecvError>>,
);

/// Component that contains task with `GameSession` streaming call and
/// channels to send/receive action data of type `T` to/from the server.
#[derive(Debug, Component)]
pub struct GameSession<T> {
    task: Option<Task<()>>,
    action_sender: async_channel::Sender<T>,
    update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
}

impl<T> GameSession<T> {
    pub fn new(
        task: Task<()>,
        action_sender: async_channel::Sender<T>,
        update_receiver: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    ) -> Self {
        Self {
            task: Some(task),
            action_sender,
            update_receiver,
        }
    }

    pub fn action_sender(&self) -> async_channel::Sender<T> {
        self.action_sender.clone()
    }

    pub fn update_receiver(&self) -> async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>> {
        self.update_receiver.clone()
    }

    pub fn task(&self) -> Option<&Task<()>> {
        self.task.as_ref()
    }

    /// Returns `true` if task is ready.
    pub fn poll_task(&mut self) -> bool {
        let Some(task) = &mut self.task else {
            return false;
        };
        tasks::block_on(future::poll_once(task)).is_some()
    }

    pub fn drop_task(&mut self) {
        let _ = self.task.take();
    }
}
