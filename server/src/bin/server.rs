extern crate server;

use tokio::signal;
use tokio_util::sync::CancellationToken;

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
    let mut game_impl = GameImpl::default();
    let workers = game_impl.start_workers(ct.clone());
    let server = tonic::transport::Server::builder()
        .add_service(rpc_server::spec_service()?)
        .add_service(GameServer::new(game_impl))
        .serve_with_shutdown(addr, async move {
            if let Err(err) = workers.await {
                println!("workers join task failed: {}", err);
            };
        });

    if let Err(err) = signal::ctrl_c().await {
        println!("unable to listen for shutdown signal: {}", err);
    }
    ct.cancel();
    server.await?;

    Ok(())
}
