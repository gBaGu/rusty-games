extern crate server;

use std::net::{Ipv6Addr, SocketAddr};
use std::{fs, io, path};

use clap::Parser;
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

fn parse_hash(s: &str) -> Result<Hmac<sha2::Sha256>, String> {
    let bytes = hex::decode(s).map_err(|e| e.to_string())?;
    Hmac::new_from_slice(&bytes).map_err(|e| e.to_string())
}

#[derive(Debug, Parser)]
struct Args {
    /// Network port to use
    #[arg(short, long)]
    port: u16,
    /// Path to OAuth2.0 configuration file
    #[arg(long)]
    oauth2_settings_path: path::PathBuf,
    /// PostgreSQL connection string
    #[arg(long, env)]
    database_url: String,
    /// Private key to use for JWT signing
    #[arg(long, env)]
    #[arg(value_parser = parse_hash)]
    jwt_secret: Hmac<sha2::Sha256>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv::dotenv().ok();
    let args = Args::parse();
    let rpc_addr = SocketAddr::new(Ipv6Addr::LOCALHOST.into(), args.port);
    let oauth2_settings_file = fs::File::open(&args.oauth2_settings_path)?;
    let auth_settings: rpc_server::OAuth2Settings =
        serde_json::from_reader(io::BufReader::new(oauth2_settings_file))?;
    let Some(redirect_port) = auth_settings.redirect_url().split(':').skip(1).last() else {
        println!("redirect url doesn't contain port");
        return Ok(());
    };
    let redirect_addr = format!("[::1]:{}", redirect_port).parse()?;
    let db = db::Connection::new(&args.database_url);

    let ct = CancellationToken::new();
    let (mut health_reporter, health_service) = tonic_health::server::health_reporter();
    let mut game_impl = rpc_server::GameImpl::default();
    let game_workers = game_impl.start_workers(ct.clone());
    let mut auth_impl = rpc_server::AuthImpl::new(auth_settings, db);
    let auth_workers = auth_impl.start(args.jwt_secret.clone(), redirect_addr, ct.clone());
    let shutdown_input_task = tokio::spawn(listen_ctrl_c(ct.clone()));
    let shutdown_signal = async move {
        ct.cancelled().await;
        health_reporter
            .set_service_status("", ServingStatus::NotServing)
            .await;
        health_reporter.clear_service_status("").await;
        if let Err(err) = game_workers.await {
            println!("unable to wait for game workers to shutdown: {}", err);
        };
        if let Err(err) = auth_workers.await {
            println!("unable to wait for auth workers to shutdown: {}", err);
        };
    };
    println!("listening for connections on {}", rpc_addr);
    tonic::transport::Server::builder()
        .add_service(rpc_server::spec_service()?)
        .add_service(health_service)
        .add_service(GameServer::with_interceptor(
            game_impl,
            rpc_server::ValidateJWT::new(args.jwt_secret),
        ))
        .add_service(AuthServer::new(auth_impl))
        .serve_with_shutdown(rpc_addr, shutdown_signal)
        .await?;
    shutdown_input_task.await?;

    Ok(())
}
