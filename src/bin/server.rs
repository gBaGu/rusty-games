extern crate mp_game;

use mp_game::rpc_server::rpc::{game_proto::{game_server::GameServer, FILE_DESCRIPTOR_SET}, GameService};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let game_service = GameService::default();
    let reflect_service = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()?;

    tonic::transport::Server::builder()
        .add_service(GameServer::new(game_service))
        .add_service(reflect_service)
        .serve(addr)
        .await?;

    Ok(())
}