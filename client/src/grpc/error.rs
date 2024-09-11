use game_server::core::ProtobufError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GrpcError {
    #[error("grpc client is not connected")]
    NotConnected,
    #[error(transparent)]
    ProtobufConversion(#[from] ProtobufError),
}
