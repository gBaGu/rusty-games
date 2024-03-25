extern crate mp_game;

use mp_game::rpc_server;

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

    tonic::transport::Server::builder()
        .add_service(rpc_server::spec_service()?)
        .add_service(rpc_server::game_service())
        .serve(addr)
        .await?;

    Ok(())
}
