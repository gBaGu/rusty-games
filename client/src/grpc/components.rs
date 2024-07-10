use bevy::prelude::*;
use bevy::tasks::Task;
use tonic::transport;
use game_server::proto;
use game_server::rpc_server::rpc::RpcResult;

/// Task component for connecting to grpc server
#[derive(Component, Deref, DerefMut)]
pub struct ConnectClient(pub Task<Result<transport::Channel, transport::Error>>);

/// Task component for creating game over grpc
#[derive(Component, Deref, DerefMut)]
pub struct CallCreateGame(pub Task<RpcResult<proto::CreateGameReply>>);

/// Task component for updating game over grpc
#[derive(Component, Deref, DerefMut)]
pub struct CallMakeTurn(pub Task<RpcResult<proto::MakeTurnReply>>);

/// Task component for loading full game data over grpc
#[derive(Component, Deref, DerefMut)]
pub struct CallGetGame(pub Task<RpcResult<proto::GetGameReply>>);

/// Task component for loading player games over grpc
#[derive(Component, Deref, DerefMut)]
pub struct CallGetPlayerGames(pub Task<RpcResult<proto::GetPlayerGamesReply>>);

/// Task component for receiving grpc connection status
#[derive(Component, Deref, DerefMut)]
pub struct ReceiveUpdate(pub Task<Result<bool, async_channel::RecvError>>);