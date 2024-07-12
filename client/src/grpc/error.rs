use bevy::utils::thiserror;
use game_server::game::encoding::ProtobufError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GrpcError {
    #[error("grpc client is not connected")]
    NotConnected,
    #[error(transparent)]
    ProtobufConversion(#[from] ProtobufError),
}