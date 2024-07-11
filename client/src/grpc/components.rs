use bevy::prelude::*;
use bevy::tasks::Task;
use tonic::transport;
use game_server::rpc_server::rpc::RpcResult;

/// Task component for connecting to grpc server
#[derive(Component, Deref, DerefMut)]
pub struct ConnectClient(pub Task<Result<transport::Channel, transport::Error>>);

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
pub struct ReceiveUpdate(pub Task<Result<bool, async_channel::RecvError>>);
