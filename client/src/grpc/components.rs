use bevy::prelude::*;
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
pub struct ReceiveUpdateTask(pub Task<Result<bool, async_channel::RecvError>>);

#[derive(Component, Deref, DerefMut)]
pub struct GameSessionTask(Task<()>);

/// Component that contains task with `GameSession` streaming call and
/// channels to send/receive data to/from the server.
#[derive(Debug, Component)]
pub struct GameSession<T> {
    task: Task<()>,
    action_channel: async_channel::Sender<T>,
    update_channel: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
}

impl<T> GameSession<T> {
    pub fn new(
        task: Task<()>,
        action_channel: async_channel::Sender<T>,
        update_channel: async_channel::Receiver<GrpcResult<GameSessionUpdate<T>>>,
    ) -> Self {
        Self {
            task,
            action_channel,
            update_channel,
        }
    }
}
