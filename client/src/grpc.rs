use std::future::Future;
use std::ops::DerefMut;
use std::time::Duration;

use async_compat::CompatExt;
use bevy::prelude::{
    Commands, Component, Deref, DerefMut, Entity, Res, ResMut, Resource, TimerMode,
};
use bevy::tasks::{block_on, futures_lite::future, IoTaskPool, Task};
use bevy::time::{Time, Timer};
use game_server::proto;
use game_server::rpc_server::rpc::RpcResult;
use prost::Message;
use tonic::transport;
use tonic::Request;

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "http://localhost:50051";
pub const RECONNECT_INTERVAL_SEC: f32 = 5.0;

pub type GameClient = proto::game_client::GameClient<transport::Channel>;

#[derive(Debug, Default, Deref, DerefMut, Resource)]
pub struct GrpcClient {
    #[deref]
    inner: Option<GameClient>,
    connect: Option<Task<Result<GameClient, transport::Error>>>,
}

impl GrpcClient {
    pub fn from_game_client(client: GameClient) -> Self {
        Self {
            inner: Some(client),
            connect: None,
        }
    }

    pub fn create_game(
        &self,
        player_id: u64,
        opponent_id: u64,
    ) -> Option<Task<RpcResult<proto::CreateGameReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
        let task = IoTaskPool::get().spawn(async move {
            client
                .create_game(Request::new(proto::CreateGameRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    player_ids: vec![player_id, opponent_id],
                }))
                .await
        });
        Some(task)
    }

    pub fn make_turn(
        &self,
        game_id: u64,
        player_id: u64,
        row: u32,
        col: u32,
    ) -> Option<Task<RpcResult<proto::MakeTurnReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
        let position = proto::Position { row, col };
        let task = IoTaskPool::get().spawn(async move {
            client
                .make_turn(Request::new(proto::MakeTurnRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    game_id,
                    player_id,
                    turn_data: position.encode_to_vec(),
                }))
                .await
        });
        Some(task)
    }

    pub fn load_game(&self, game_id: u64) -> Option<Task<RpcResult<proto::GetGameReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_game(Request::new(proto::GetGameRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    game_id,
                }))
                .await
        });
        Some(task)
    }

    pub fn load_player_games(
        &self,
        player_id: u64,
    ) -> Option<Task<RpcResult<proto::GetPlayerGamesReply>>> {
        let mut client = self.inner.as_ref().cloned()?;
        let task = IoTaskPool::get().spawn(async move {
            client
                .get_player_games(Request::new(proto::GetPlayerGamesRequest {
                    game_type: proto::GameType::TicTacToe.into(),
                    player_id,
                }))
                .await
        });
        Some(task)
    }
}

#[derive(Deref, DerefMut, Resource)]
pub struct ReconnectTimer(pub Timer);

impl Default for ReconnectTimer {
    fn default() -> Self {
        let mut t = Timer::from_seconds(RECONNECT_INTERVAL_SEC, TimerMode::Repeating);
        t.set_elapsed(Duration::from_secs_f32(RECONNECT_INTERVAL_SEC));
        Self(t)
    }
}

/// This struct is intended for use with entities that only needed to wait on a task.
/// It makes sure that task entity is despawned after successful poll
pub struct TaskEntity<'a, 'w, 's, F> {
    commands: Commands<'w, 's>,
    entity: Entity,
    task: &'a mut F,
}

impl<'a, 'w, 's, F> TaskEntity<'a, 'w, 's, F> {
    pub fn new(commands: Commands<'w, 's>, entity: Entity, task: &'a mut F) -> Self {
        Self {
            commands,
            entity,
            task,
        }
    }
}

impl<'a, 'w, 's, F, T> TaskEntity<'_, '_, '_, F>
where
    F: Future<Output = T> + Unpin,
{
    /// Polls future and in case if result is ready adds despawn command to `commands`
    pub fn poll_once(&mut self) -> Option<T> {
        block_on(future::poll_once(self.task.deref_mut())).and_then(|res| {
            self.commands.entity(self.entity).despawn();
            Some(res)
        })
    }
}

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

pub fn reconnect(
    mut client: ResMut<GrpcClient>,
    mut timer: ResMut<ReconnectTimer>,
    time: Res<Time>,
) {
    if client.is_some() {
        timer.reset();
        return;
    }

    if timer.tick(time.delta()).just_finished() && client.connect.is_none() {
        println!("reconnecting to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            GameClient::connect(DEFAULT_GRPC_SERVER_ADDRESS)
                .compat()
                .await
        });
        client.connect = Some(task);
    }
}

pub fn handle_reconnect(mut client: ResMut<GrpcClient>) {
    if let Some(task) = &mut client.connect {
        if let Some(res) = block_on(future::poll_once(task)) {
            match res {
                Ok(c) => *client = GrpcClient::from_game_client(c),
                Err(err) => {
                    println!("grpc client connect failed: {:?}", err);
                    client.connect.take();
                }
            }
        }
    }
}
