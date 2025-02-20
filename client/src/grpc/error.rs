use async_channel::SendError;
use game_server::core::ProtobufError;
use tonic::{Code, Status};

#[derive(thiserror::Error, Debug, PartialEq)]
pub enum GrpcError {
    #[error("grpc client is not connected")]
    NotConnected,
    #[error("game session update request returned with error: {0}")]
    GameSessionUpdateFailed(String),
    #[error("reply isn't valid: {0}")]
    InvalidReply(String),
    #[error("reply stream has finished unexpectedly")]
    ReplyStreamFinished,
    #[error("request failed with {}, message: {}", .code, .message)]
    RequestFailed {
        code: Code,
        message: String,
    },
    #[error("internal error: {0}")]
    Internal(String),
    #[error(transparent)]
    ChannelRecv(#[from] async_channel::RecvError),
    #[error("{0}")]
    ChannelSend(String),
    #[error(transparent)]
    ProtobufConversion(#[from] ProtobufError),
}

impl From<Status> for GrpcError {
    fn from(value: Status) -> Self {
        Self::RequestFailed {
            code: value.code(),
            message: value.message().to_string(),
        }
    }
}

impl<T> From<SendError<T>> for GrpcError {
    fn from(value: SendError<T>) -> Self {
        Self::ChannelSend(value.to_string())
    }
}
