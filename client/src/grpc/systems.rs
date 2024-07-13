use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use tonic::transport;

use super::components::{CallTask, ConnectClientTask, ReceiveUpdateTask};
use super::events::RpcResultReady;
use super::resources::{ConnectTimer, ConnectionStatusWatcher};
use super::task_entity::TaskEntity;
use super::{
    Connected, Disconnected, GameClient, GrpcClient, HealthClient, DEFAULT_GRPC_SERVER_ADDRESS,
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
        commands.spawn(ReceiveUpdateTask(task));
    } else {
        println!("ConnectStatusWatcher is finished");
        commands.remove_resource::<ConnectionStatusWatcher>();
    }
}

pub fn handle_receive_status(
    mut commands: Commands,
    mut receive_update_task: Query<(Entity, &mut ReceiveUpdateTask)>,
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
