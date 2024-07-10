use async_compat::CompatExt;
use bevy::prelude::*;
use bevy::tasks::IoTaskPool;
use tonic::transport;

use super::components::{ConnectClient, ReceiveUpdate};
use super::resources::{ConnectTimer, ConnectionStatusWatcher};
use super::{GameClient, GrpcClient, HealthClient, TaskEntity, DEFAULT_GRPC_SERVER_ADDRESS};

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
                let client = GrpcClient::new(GameClient::new(c.clone()));
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
