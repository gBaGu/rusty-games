use tonic_reflection::server::{Error, ServerReflection, ServerReflectionServer};
use rpc::game_proto::game_server::{Game, GameServer};

pub mod rpc;

pub fn spec_service() -> Result<ServerReflectionServer<impl ServerReflection>, Error> {
    let spec = tonic_reflection::server::Builder::configure()
        .register_encoded_file_descriptor_set(rpc::game_proto::FILE_DESCRIPTOR_SET)
        .build()?;
    Ok(spec)
}

pub fn game_service() -> GameServer<impl Game> {
    GameServer::new(rpc::GameImpl::default())
}