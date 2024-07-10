use std::future::Future;
use std::ops::DerefMut;
use std::time::Duration;

use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks::{block_on, futures_lite::future, IoTaskPool, Task};
use game_server::proto;
use game_server::rpc_server::rpc::RpcResult;
use prost::Message;
use tonic::codegen::tokio_stream::StreamExt;
use tonic::{transport, Code, Request};
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::{health_client, HealthCheckRequest};

pub const DEFAULT_GRPC_SERVER_ADDRESS: &str = "http://localhost:50051";
pub const CONNECT_INTERVAL_SEC: f32 = 5.0;
pub const HEALTH_RETRY_INTERVAL_SEC: f32 = 5.0;

pub fn client_exists_and_connected(client: Option<Res<GrpcClient>>) -> bool {
    matches!(client, Some(c) if c.connected)
}

/// System set for network communication systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct NetworkSystems;

pub type GameClient = proto::game_client::GameClient<transport::Channel>;
pub type HealthClient = health_client::HealthClient<transport::Channel>;

#[derive(Debug, Resource)]
pub struct GrpcClient {
    game: GameClient,
    connected: bool,
}

impl GrpcClient {
    pub fn set_connected(&mut self, connected: bool) {
        if self.connected != connected {
            println!("connection status changed: {}", connected);
        }
        self.connected = connected;
    }

    pub fn create_game(
        &self,
        player_id: u64,
        opponent_id: u64,
    ) -> Option<Task<RpcResult<proto::CreateGameReply>>> {
        let mut client = self.game.clone();
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
        let mut client = self.game.clone();
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
        let mut client = self.game.clone();
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
        let mut client = self.game.clone();
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

#[derive(Resource)]
pub struct ConnectionStatusWatcher {
    update_receiver: async_channel::Receiver<bool>,
}

impl ConnectionStatusWatcher {
    pub fn start(mut health_client: HealthClient) -> Self {
        let (s, r) = async_channel::unbounded();
        let watch_task = IoTaskPool::get().spawn(async move {
            loop {
                let res = health_client
                    .watch(Request::new(HealthCheckRequest {
                        service: "".to_string(),
                    }))
                    .await;
                match res {
                    Ok(response) => {
                        let mut stream = response.into_inner();
                        while let Some(res) = stream.next().await {
                            let status = match res {
                                Ok(response) => {
                                    response.status
                                        == <ServingStatus as Into<i32>>::into(
                                            ServingStatus::Serving,
                                        )
                                }
                                Err(err) => {
                                    println!("got error from health client watch stream: {}", err);
                                    false
                                }
                            };
                            if let Err(err) = s.send(status).await {
                                println!("unable to send connection status update: {}", err);
                            }
                        }
                    }
                    Err(status) if status.code() == Code::Unimplemented => break,
                    Err(status) => {
                        println!("got error from health service: {}", status);
                        if let Err(err) = s.send(false).await {
                            println!("unable to send connection status update: {}", err);
                        }
                    }
                }
                async_io::Timer::after(Duration::from_secs_f32(HEALTH_RETRY_INTERVAL_SEC)).await;
            }
        });
        watch_task.detach();
        Self { update_receiver: r }
    }

    pub fn update_receiver(&self) -> async_channel::Receiver<bool> {
        self.update_receiver.clone()
    }
}

#[derive(Deref, DerefMut, Resource)]
pub struct ConnectTimer(pub Timer);

impl Default for ConnectTimer {
    fn default() -> Self {
        let mut t = Timer::from_seconds(CONNECT_INTERVAL_SEC, TimerMode::Repeating);
        t.set_elapsed(Duration::from_secs_f32(CONNECT_INTERVAL_SEC));
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

pub fn connect(mut commands: Commands, mut timer: ResMut<ConnectTimer>, time: Res<Time>) {
    if timer.tick(time.delta()).just_finished() {
        println!("trying to connect to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            transport::Endpoint::new(DEFAULT_GRPC_SERVER_ADDRESS)?
                .connect()
                .compat()
                .await
        });
        commands.spawn(ConnectClient(task));
    }
}

pub fn handle_connect(
    mut commands: Commands,
    mut connect_task: Query<(Entity, &mut ConnectClient)>,
    client: Option<Res<GrpcClient>>,
) {
    let Ok((entity, mut task)) = connect_task.get_single_mut() else {
        println!("unable to get a single connect task");
        return;
    };
    let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task.0);
    if let Some(res) = task.poll_once() {
        if client.is_some() {
            return;
        }
        match res {
            Ok(c) => {
                let client = GrpcClient {
                    game: GameClient::new(c.clone()),
                    connected: true,
                };
                println!("server connection established, creating health watcher");
                let watcher = ConnectionStatusWatcher::start(HealthClient::new(c));
                commands.insert_resource(client);
                commands.insert_resource(watcher);
            }
            Err(err) => {
                println!("grpc client connect failed: {:?}", err);
            }
        }
    }
}

pub fn receive_status(mut commands: Commands, watcher: Res<ConnectionStatusWatcher>) {
    let receiver = watcher.update_receiver();
    if !receiver.is_closed() {
        let task = IoTaskPool::get().spawn(async move { receiver.recv().await });
        commands.spawn(ReceiveUpdate(task));
    } else {
        println!("ConnectStatusWatcher is finished");
        commands.remove_resource::<ConnectionStatusWatcher>();
    }
}

pub fn handle_receive_status(
    mut commands: Commands,
    mut receive_update_task: Query<(Entity, &mut ReceiveUpdate)>,
    client: Option<ResMut<GrpcClient>>,
) {
    let Ok((entity, mut task)) = receive_update_task.get_single_mut() else {
        println!("unable to get a single connect task");
        return;
    };
    let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task.0);
    if let Some(res) = task.poll_once() {
        if let Some(mut client) = client {
            let connected = res.unwrap_or_else(|err| {
                println!("failed to get connection status: {}", err);
                false
            });
            client.set_connected(connected);
        }
    }
}
