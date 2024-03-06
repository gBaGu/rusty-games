extern crate mp_game;

use mp_game::rpc_server::rpc::GameService;
use mp_game::rpc_server::rpc::game_proto::game_server::GameServer;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051".parse()?;
    let service = GameService::default();

    tonic::transport::Server::builder()
        .add_service(GameServer::new(service))
        .serve(addr)
        .await?;

    Ok(())
}