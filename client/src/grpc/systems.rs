use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks;
use bevy::tasks::futures_lite::future;
use bevy::tasks::IoTaskPool;
use game_server::{core, proto};
use tonic::transport;

use super::components::{
    CallTask, ConnectClientTask, ReceiveConnectionStatusTask, ReceiveSessionUpdateTask,
    ReconnectSession, ReconnectSessionBundle, ReconnectSessionTimer,
};
use super::events::{RpcResultReady, SessionUpdateReceived};
use super::resources::{ConnectTimer, ConnectionStatusWatcher, SessionCheckTimer};
use super::task_entity::TaskEntity;
use super::{
    CloseSession, Connected, Disconnected, GameClient, GameSession, GrpcClient, HealthClient,
    SendActionTask, SendSessionAction, SendSessionActionFailed, SessionFinished,
    DEFAULT_GRPC_SERVER_ADDRESS,
};
use crate::game::{ActiveGame, NetworkGame};

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
        let mut task = TaskEntity::new(commands.reborrow(), entity, &mut task);
        if let Some(res) = task.poll_once() {
            response_ready.send(RpcResultReady::new(res));
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
/// and send [`SessionFinished`] event.
pub fn session_finished<T>(
    mut commands: Commands,
    mut session: Query<(
        Entity,
        &mut GameSession<T, T::TurnData>,
        Option<&ActiveGame>,
    )>,
    mut timer: ResMut<SessionCheckTimer>,
    mut session_finished: EventWriter<SessionFinished>,
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
                session_finished.send(SessionFinished::new(session_entity));
                if active.is_some() {
                    commands
                        .entity(session_entity)
                        .insert(ReconnectSessionBundle::<T>::default());
                }
            }
        }
    }
}

/// Wait till the [`ReconnectSessionTimer`] is finished and send `GetGame` request.
pub fn send_get_game_before_reconnect<T>(
    mut commands: Commands,
    mut reconnect: Query<
        (Entity, &mut ReconnectSessionTimer, &NetworkGame),
        With<ReconnectSession<T>>,
    >,
    client: Option<Res<GrpcClient>>,
    time: Res<Time>,
) where
    T: proto::GetGameType + Send + Sync + 'static,
{
    for (game_entity, mut timer, network_game) in reconnect.iter_mut() {
        if timer.tick(time.delta()).just_finished() {
            let Some(ref client) = client else {
                println!("unable to reconnect session: grpc client is not connected");
                commands
                    .entity(game_entity)
                    .remove::<ReconnectSessionBundle<T>>();
                continue;
            };
            match client.get_game::<T>(**network_game) {
                Ok(task) => {
                    commands.spawn(task);
                }
                Err(err) => {
                    println!("unable to reconnect session: GetGame call failed: {}", err);
                    commands
                        .entity(game_entity)
                        .remove::<ReconnectSessionBundle<T>>();
                }
            }
        }
    }
}

pub fn init_session_action_send_task<T>(
    mut commands: Commands,
    session: Query<&GameSession<T, T::TurnData>>,
    mut send_session_action: EventReader<SendSessionAction<T::TurnData>>,
) where
    T: core::Game + Send + Sync + 'static,
    T::TurnData: Copy + Send + Sync + 'static,
{
    for event in send_session_action.read() {
        let Ok(session) = session.get(event.session_entity()) else {
            continue;
        };
        let sender = session.action_sender();
        let action = *event.action();
        println!("send session action");
        let task = IoTaskPool::get().spawn(async move { sender.send(action).await });
        commands
            .entity(event.session_entity())
            .insert(SendActionTask(task));
    }
}

pub fn handle_session_action_send<T>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut SendActionTask<T>)>,
    mut send_action_failed: EventWriter<SendSessionActionFailed>,
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
                send_action_failed.send(SendSessionActionFailed::new(task_entity));
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
    // TODO: make this system run by some timer
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
                    // TODO: this means successful read from channel, not an action received
                    println!("GameSessionReply is received");
                    update_received.send(SessionUpdateReceived::<T>::new(session_entity, res));
                }
                Err(err) => {
                    println!("failed to read from session update channel: {}", err);
                }
            };
        }
    }
}
