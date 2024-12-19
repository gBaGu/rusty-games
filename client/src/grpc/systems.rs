use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks;
use bevy::tasks::futures_lite::future;
use bevy::tasks::IoTaskPool;
use tonic::transport;

use super::components::{
    CallTask, ConnectClientTask, ReceiveConnectionStatusTask, ReceiveSessionUpdateTask,
};
use super::events::{RpcResultReady, SessionUpdateReceived};
use super::resources::{ConnectTimer, ConnectionStatusWatcher, SessionCheckTimer};
use super::task_entity::TaskEntity;
use super::{
    Connected, Disconnected, GameClient, GameSession, GrpcClient, HealthClient, SendActionTask,
    SendSessionAction, SessionFinished, DEFAULT_GRPC_SERVER_ADDRESS,
};

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

/// Polls unfinished session tasks and if the task is ready drop it and send [`SessionFinished`] event.
pub fn session_finished<T: Copy + Send + Sync + 'static>(
    mut session: Query<(Entity, &mut GameSession<T>)>,
    mut timer: ResMut<SessionCheckTimer>,
    mut session_finished: EventWriter<SessionFinished>,
    time: Res<Time>,
) {
    if timer.tick(time.delta()).just_finished() {
        for (session_entity, mut session) in session.iter_mut().filter(|(_, s)| s.task().is_some())
        {
            if session.poll_task() {
                session.drop_task();
                session_finished.send(SessionFinished::new(session_entity));
            }
        }
    }
}

pub fn init_session_action_send_task<T: Copy + Send + Sync + 'static>(
    mut commands: Commands,
    session: Query<&GameSession<T>>,
    mut send_session_action: EventReader<SendSessionAction<T>>,
) {
    for event in send_session_action.read() {
        let Ok(session) = session.get(event.session_entity()) else {
            continue;
        };
        // TODO: handle a case when sending into a closed channel
        let sender = session.action_sender();
        let action = *event.action();
        println!("send session action");
        let task = IoTaskPool::get().spawn(async move { sender.send(action).await });
        commands
            .entity(event.session_entity())
            .insert(SendActionTask(task));
    }
}

pub fn handle_session_action_send<T: Send + Sync + 'static>(
    mut commands: Commands,
    mut task: Query<(Entity, &mut SendActionTask<T>)>,
) {
    for (task_entity, mut task) in task.iter_mut() {
        if let Some(res) = tasks::block_on(future::poll_once(&mut task.0)).and_then(|res| {
            commands.entity(task_entity).remove::<SendActionTask<T>>();
            Some(res)
        }) {
            println!("send session action task is finished: {:?}", res);
            if let Err(_err) = res {
                // TODO: handle error
            }
        }
    }
}

pub fn init_session_update_receive_task<T: Send + Sync + 'static>(
    mut commands: Commands,
    session: Query<(Entity, &GameSession<T>), Without<ReceiveSessionUpdateTask<T>>>,
) {
    // TODO: make this system run by some timer
    for (session_entity, session) in session.iter().filter(|(_, s)| s.task().is_some()) {
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

pub fn handle_session_update_receive<T: Send + Sync + 'static>(
    mut commands: Commands,
    mut session: Query<(Entity, &mut ReceiveSessionUpdateTask<T>)>,
    mut update_received: EventWriter<SessionUpdateReceived<T>>,
) {
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
