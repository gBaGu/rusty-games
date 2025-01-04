use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks;
use bevy::tasks::futures_lite::future;
use bevy::tasks::IoTaskPool;
use game_server::{core, proto};
use tonic::transport;

use super::components::{
    CallTask, ConnectClientTask, ConnectingGameSession, ReceiveConnectionStatusTask,
    ReceiveSessionUpdateTask, ReconnectSessionBundle, ReconnectSessionTimer,
};
use super::events::{RpcResultReady, SessionUpdateReceived};
use super::resources::{ConnectTimer, ConnectionStatusWatcher, SessionCheckTimer};
use super::task_entity::TaskEntity;
use super::{
    CloseSession, Connected, Disconnected, GameClient, GameSession, GrpcClient, HealthClient,
    OpenSession, SendActionTask, SessionActionSendFailed, SessionActionReadyToSend,
    SessionClosed, SessionOpened, DEFAULT_GRPC_SERVER_ADDRESS,
};
use crate::game::{ActiveGame, NetworkGame};
use crate::Settings;

pub fn connect(mut commands: Commands, mut timer: ResMut<ConnectTimer>, time: Res<Time>) {
    if timer.tick(time.delta()).just_finished() {
        println!("trying to connect to grpc server...");
        let task = IoTaskPool::get().spawn(async move {
            transport::Endpoint::new(DEFAULT_GRPC_SERVER_ADDRESS)?
                .connect()
                .compat()
                .await
        });
        commands.spawn(ConnectClientTask(task));
    }
}

pub fn handle_connect(
    mut commands: Commands,
    mut connect_task: Query<(Entity, &mut ConnectClientTask)>,
    mut connected: EventWriter<Connected>,
    client: Option<Res<GrpcClient>>,
) {
    let Ok((entity, mut task)) = connect_task.get_single_mut() else {
        println!("unable to get a single connect task");
        return;
    };
    let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task);
    if let Some(res) = task.poll_once() {
        if client.is_some() {
            return;
        }
        match res {
            Ok(c) => {
                let client = GrpcClient::new(GameClient::new(c.clone()));
                println!("server connection established, creating health watcher");
                let watcher = ConnectionStatusWatcher::start(HealthClient::new(c));
                commands.insert_resource(client);
                commands.insert_resource(watcher);
                connected.send(Connected);
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
        commands.spawn(ReceiveConnectionStatusTask(task));
    } else {
        println!("ConnectStatusWatcher is finished");
        commands.remove_resource::<ConnectionStatusWatcher>();
    }
}

pub fn handle_receive_status(
    mut commands: Commands,
    mut receive_update_task: Query<(Entity, &mut ReceiveConnectionStatusTask)>,
    mut connected: EventWriter<Connected>,
    mut disconnected: EventWriter<Disconnected>,
    client: Option<ResMut<GrpcClient>>,
) {
    let Ok((entity, mut task)) = receive_update_task.get_single_mut() else {
        println!("unable to get a single connect task");
        return;
    };
    let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task);
    if let Some(res) = task.poll_once() {
        if let Some(mut client) = client {
            let updated_status = res.unwrap_or_else(|err| {
                println!("failed to get connection status: {}", err);
                false
            });
            if client.connected() && !updated_status {
                println!("grpc client disconnected");
                disconnected.send(Disconnected);
            } else if !client.connected() && updated_status {
                println!("grpc client connected");
                connected.send(Connected);
            }
            client.set_connected(updated_status);
        }
    }
}

pub fn handle_response<T: Send + Sync + 'static>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut CallTask<T>)>,
    mut response_ready: EventWriter<RpcResultReady<T>>,
) {
    for (entity, mut task) in task.iter_mut() {
        if let Some(res) = tasks::block_on(future::poll_once(&mut **task)) {
            commands.entity(entity).remove::<CallTask<T>>();
            response_ready.send(RpcResultReady::new(entity, res));
        }
    }
}

/// Listen to [`CloseSession`] event and close input channel of the [`GameSession`].
pub fn close_session<T>(
    session: Query<&GameSession<T, T::TurnData>>,
    mut close_session: EventReader<CloseSession>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    for event in close_session.read() {
        if let Ok(session) = session.get(**event) {
            session.action_sender().close();
        }
    }
}

/// Polls unfinished session tasks and if the task is ready remove [`GameSession`] from the entity
/// and send [`SessionClosed`] event.
pub fn session_closed<T>(
    mut commands: Commands,
    mut session: Query<(
        Entity,
        &mut GameSession<T, T::TurnData>,
        Option<&ActiveGame>,
    )>,
    mut timer: ResMut<SessionCheckTimer>,
    mut session_closed: EventWriter<SessionClosed>,
    mut open_session: EventWriter<OpenSession>,
    time: Res<Time>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    if timer.tick(time.delta()).just_finished() {
        for (session_entity, mut session, active) in session.iter_mut() {
            if tasks::block_on(future::poll_once(session.task_mut())).is_some() {
                commands
                    .entity(session_entity)
                    .remove::<GameSession<T, T::TurnData>>();
                session_closed.send(SessionClosed::new(session_entity));
                if active.is_some() {
                    open_session.send(OpenSession::new_delayed(session_entity));
                }
            }
        }
    }
}

/// Receive the [`OpenSession`] event and insert components required for session initialization
/// into the entity contained in the event.
pub fn init_open_session<T>(mut commands: Commands, mut open_session: EventReader<OpenSession>)
where
    T: Send + Sync + 'static,
{
    for event in open_session.read() {
        let mut session_cmds = commands.entity(event.game());
        if event.delayed() {
            session_cmds.insert(ReconnectSessionBundle::<T>::default());
        } else {
            session_cmds.insert(ConnectingGameSession::<T>::default());
        }
    }
}

/// Update [`ReconnectSessionTimer`] component and remove it once finished.
pub fn delay_session_connection(
    mut commands: Commands,
    mut timer: Query<(Entity, &mut ReconnectSessionTimer)>,
    time: Res<Time>,
) {
    for (session_entity, mut timer) in timer.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            commands
                .entity(session_entity)
                .remove::<ReconnectSessionTimer>();
        }
    }
}

/// If session entity doesn't have [`ReconnectSessionTimer`] component send `GetGame` request.
pub fn send_get_game_before_connect<T>(
    mut commands: Commands,
    mut connecting_session: Query<
        (Entity, &NetworkGame),
        (
            With<ConnectingGameSession<T>>,
            Without<ReconnectSessionTimer>,
            Without<CallTask<proto::GetGameReply>>,
        ),
    >,
    client: Option<Res<GrpcClient>>,
) where
    T: proto::GetGameType + Send + Sync + 'static,
{
    for (game_entity, network_game) in connecting_session.iter_mut() {
        let Some(ref client) = client else {
            println!("unable to reconnect session: grpc client is not connected");
            commands
                .entity(game_entity)
                .remove::<ConnectingGameSession<T>>();
            continue;
        };
        match client.get_game::<T>(**network_game) {
            Ok(task) => {
                commands.entity(game_entity).insert(task);
            }
            Err(err) => {
                println!("unable to reconnect session: GetGame call failed: {}", err);
                commands
                    .entity(game_entity)
                    .remove::<ConnectingGameSession<T>>();
            }
        }
    }
}

/// Receive `GetGameReply` and if session entity doesn't have [`ReconnectSessionTimer`] component
/// start game session by sending grpc request.
/// On success insert game session component into the game entity and send [`SessionOpened`] event.
pub fn connect_session<T>(
    mut commands: Commands,
    connecting_session: Query<
        &NetworkGame,
        (
            With<ConnectingGameSession<T>>,
            Without<ReconnectSessionTimer>,
        ),
    >,
    mut get_game_reply: EventReader<RpcResultReady<proto::GetGameReply>>,
    mut session_opened: EventWriter<SessionOpened>,
    client: Option<Res<GrpcClient>>,
    settings: Res<Settings>,
) where
    T: core::Game + proto::GetGameType + Send + Sync + 'static,
    T::TurnData: Send + 'static,
{
    for event in get_game_reply.read() {
        let Ok(network_game) = connecting_session.get(event.entity()) else {
            continue;
        };
        commands
            .entity(event.entity())
            .remove::<ConnectingGameSession<T>>();
        let Some(user) = settings.user_id() else {
            println!("unable to reconnect session: user is not logged in");
            continue;
        };
        let Some(ref client) = client else {
            println!("unable to reconnect session: grpc client is not connected");
            continue;
        };
        match event.result() {
            Ok(_) => match client.game_session::<T>(**network_game, user) {
                Ok(session) => {
                    commands.entity(event.entity()).insert(session);
                    session_opened.send(SessionOpened::new(event.entity()));
                }
                Err(err) => println!(
                    "unable to reconnect session: GameSession call failed: {}",
                    err
                ),
            },
            Err(err) => {
                println!("unable to reconnect session: GetGame call failed: {}", err);
            }
        }
    }
}

/// Receive the [`SessionActionReadyToSend`] event, find [`GameSession`] entity and
/// create a task that will send the action from event into session sender.
/// Send the [`SessionActionSendFailed`] event if there is no [`GameSession`] entity.
pub fn init_session_action_send_task<T>(
    mut commands: Commands,
    session: Query<&GameSession<T, T::TurnData>>,
    mut action_ready: EventReader<SessionActionReadyToSend<T::TurnData>>,
    mut action_send_failed: EventWriter<SessionActionSendFailed>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Copy + Send + Sync + 'static,
{
    for event in action_ready.read() {
        let Ok(session) = session.get(event.session_entity()) else {
            println!("failed to send session action: session component not found");
            action_send_failed.send(SessionActionSendFailed::new(event.session_entity()));
            continue;
        };
        let sender = session.action_sender();
        let action = *event.action();
        let task = IoTaskPool::get().spawn(async move { sender.send(action).await });
        commands
            .entity(event.session_entity())
            .insert(SendActionTask(task));
    }
}

/// Poll channel send task. If it returned with error, send the [`SessionActionSendFailed`] event.
pub fn handle_session_action_send<T>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut SendActionTask<T>)>,
    mut action_send_failed: EventWriter<SessionActionSendFailed>,
) where
    T: Send + Sync + 'static,
{
    for (task_entity, mut task) in task.iter_mut() {
        if let Some(res) = tasks::block_on(future::poll_once(&mut task.0)).and_then(|res| {
            commands.entity(task_entity).remove::<SendActionTask<T>>();
            Some(res)
        }) {
            if let Err(err) = res {
                println!("send session action task failed: {}", err);
                action_send_failed.send(SessionActionSendFailed::new(task_entity));
            }
        }
    }
}

pub fn init_session_update_receive_task<T>(
    mut commands: Commands,
    session: Query<
        (Entity, &GameSession<T, T::TurnData>),
        Without<ReceiveSessionUpdateTask<T::TurnData>>,
    >,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Send,
{
    for (session_entity, session) in session.iter() {
        let receiver = session.update_receiver();
        if !receiver.is_closed() {
            println!("create receive session update task");
            let task = IoTaskPool::get().spawn(async move { receiver.recv().await });
            commands
                .entity(session_entity)
                .insert(ReceiveSessionUpdateTask(task));
        }
    }
}

pub fn handle_session_update_receive<T>(
    mut commands: Commands,
    mut session: Query<(Entity, &mut ReceiveSessionUpdateTask<T>)>,
    mut update_received: EventWriter<SessionUpdateReceived<T>>,
) where
    T: Send + Sync + 'static,
{
    for (session_entity, mut task) in session.iter_mut() {
        if let Some(res) = tasks::block_on(future::poll_once(&mut task.0)).and_then(|res| {
            commands
                .entity(session_entity)
                .remove::<ReceiveSessionUpdateTask<T>>();
            Some(res)
        }) {
            match res {
                Ok(res) => {
                    update_received.send(SessionUpdateReceived::<T>::new(session_entity, res));
                }
                Err(err) => {
                    println!("failed to read from session update channel: {}", err);
                }
            };
        }
    }
}
