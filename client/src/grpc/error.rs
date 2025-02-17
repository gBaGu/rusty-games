use game_server::core::ProtobufError;

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GrpcError {
    #[error("grpc client is not connected")]
    NotConnected,
    #[error("game session update request returned with error: {0}")]
    GameSessionUpdateFailed(String),
    #[error(transparent)]
    ChannelRecv(#[from] async_channel::RecvError),
    #[error(transparent)]
    ProtobufConversion(#[from] ProtobufError),
}
