extern crate server;

use tokio::signal;
use tokio_util::sync::CancellationToken;
use tonic_health::ServingStatus;

use server::proto::game_server::GameServer;
use server::rpc_server;
use server::rpc_server::rpc::GameImpl;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 1 {
        println!("no port provided");
        return Ok(());
    }
    let port = &args[0];
    let addr = format!("[::1]:{}", port).parse()?;
    println!("listening for connections on {}", addr);

    let ct = CancellationToken::new();
    let ct_cloned = ct.clone();
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    let health = async move {
        ct_cloned.cancelled().await;
        health_reporter.set_service_status("", ServingStatus::NotServing).await;
        health_reporter.clear_service_status("").await;
    };
    let mut game_impl = GameImpl::default();
    let workers = game_impl.start_workers(ct.clone());
    let shutdown_signal = async move {
        health.await;
        if let Err(err) = workers.await {
            println!("unable to wait for workers to shutdown: {}", err);
        };
    };
    let shutdown_input_task = tokio::spawn(async move {
        if let Err(err) = signal::ctrl_c().await {
            println!("unable to listen for shutdown signal: {}", err);
        }
        ct.cancel();
    });
    tonic::transport::Server::builder()
        .add_service(rpc_server::spec_service()?)
        .add_service(health_service)
        .add_service(GameServer::new(game_impl))
        .serve_with_shutdown(addr, shutdown_signal)
        .await?;
    shutdown_input_task.await?;

    Ok(())
}
