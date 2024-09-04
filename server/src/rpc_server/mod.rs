pub mod rpc;

mod error;
mod game_storage;
mod lobby;
mod lobby_manager;

use tonic_reflection::server::{Builder, Error, ServerReflection, ServerReflectionServer};

use crate::proto::FILE_DESCRIPTOR_SET;

pub use rpc::GameId;

pub fn spec_service() -> Result<ServerReflectionServer<impl ServerReflection>, Error> {
    let spec = Builder::configure()
        .register_encoded_file_descriptor_set(FILE_DESCRIPTOR_SET)
        .build_v1()?;
    Ok(spec)
}
