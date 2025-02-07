extern crate server;

use std::fs::File;
use std::io::BufReader;

use hmac::{Hmac, Mac};
use tokio::signal;
use tokio_util::sync::CancellationToken;
use tonic_health::ServingStatus;

use server::proto::auth_server::AuthServer;
use server::proto::game_server::GameServer;
use server::{db, rpc_server};

async fn listen_ctrl_c(ct: CancellationToken) {
    if let Err(err) = signal::ctrl_c().await {
        println!("unable to listen for shutdown signal: {}", err);
    }
    ct.cancel();
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        println!("usage: server <rpc-port> <oauth2-settings.json> <secret-file>");
        return Ok(());
    }
    let rpc_port = &args[0];
    let rpc_addr = format!("[::1]:{}", rpc_port).parse()?;
    let oauth2_settings_file = File::open(&args[1])?;
    let auth_settings: rpc_server::OAuth2Settings =
        serde_json::from_reader(BufReader::new(oauth2_settings_file))?;
    let Ok(Ok(secret)) = std::fs::read_to_string(&args[2]).and_then(|s| Ok(hex::decode(s))) else {
        println!("unable to read secret");
        return Ok(());
    };
    let secret = Hmac::new_from_slice(&secret).unwrap();
    let Some(redirect_port) = auth_settings.redirect_url().split(':').skip(1).last() else {
        println!("redirect url doesn't contain port");
        return Ok(());
    };
    let redirect_addr = format!("[::1]:{}", redirect_port).parse()?;
    println!("listening for connections on {}", rpc_addr);

    let ct = CancellationToken::new();
    let ct_cloned = ct.clone();
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    let health = async move {
        ct_cloned.cancelled().await;
        health_reporter
            .set_service_status("", ServingStatus::NotServing)
            .await;
        health_reporter.clear_service_status("").await;
    };
    let mut game_impl = rpc_server::GameImpl::default();
    let game_workers = game_impl.start_workers(ct.clone());
    let mut auth_impl = rpc_server::AuthImpl::new(auth_settings, db::Connection::new());
    let auth_workers = auth_impl.start(secret.clone(), redirect_addr, ct.clone());
    let shutdown_signal = async move {
        health.await;
        if let Err(err) = game_workers.await {
            println!("unable to wait for game workers to shutdown: {}", err);
        };
        if let Err(err) = auth_workers.await {
            println!("unable to wait for auth workers to shutdown: {}", err);
        };
    };
    let shutdown_input_task = tokio::spawn(listen_ctrl_c(ct));
    tonic::transport::Server::builder()
        .add_service(rpc_server::spec_service()?)
        .add_service(health_service)
        .add_service(GameServer::with_interceptor(
            game_impl,
            rpc_server::ValidateJWT::new(secret),
        ))
        .add_service(AuthServer::new(auth_impl))
        .serve_with_shutdown(rpc_addr, shutdown_signal)
        .await?;
    shutdown_input_task.await?;

    Ok(())
}
