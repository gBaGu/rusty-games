use bevy::prelude::*;
use game_server::rpc_server::rpc::RpcResult;

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
