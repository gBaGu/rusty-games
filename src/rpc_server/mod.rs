pub mod rpc;

mod game_proto;
mod game_storage;

use tonic_reflection::server::{Builder, Error, ServerReflection, ServerReflectionServer};

use game_proto::{
    game_server::{Game, GameServer},
    FILE_DESCRIPTOR_SET,
};

pub fn spec_service() -> Result<ServerReflectionServer<impl ServerReflection>, Error> {
    let spec = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build()?;
    Ok(spec)
}

pub fn game_service() -> GameServer<impl Game> {
    GameServer::new(rpc::GameImpl::default())
}
