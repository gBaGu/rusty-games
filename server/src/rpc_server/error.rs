use std::sync::PoisonError;

use tokio::sync::mpsc::error::SendError;
use tonic::Status;

use super::GameId;
use crate::game::encoding::ProtobufError;
use crate::game::error::GameError;

#[derive(thiserror::Error, Debug)]
pub enum RpcError {
    #[error("game must be finished before deletion")]
    DeleteActiveGameFailed,
    #[error("this player already has an active game")]
    DuplicateGame,
    #[error("unrecognized game type")]
    InvalidGameType,
    #[error("game with this id doesn't exist: {id}")]
    NoSuchGame { id: GameId },
    #[error("player trying to access game they doesn't belong to")]
    ForeignGame,
    #[error("internal error: {reason}")]
    Internal { reason: String },
    #[error("failed to lock inner mutex: {description}")]
    MutexPoison { description: String },
    #[error("invalid turn data: {source}")]
    TurnDataConversion {
        #[from]
        source: ProtobufError,
    },
    #[error("failed to send data over channel: {reason}")]
    ChannelSendFailed { reason: String },
    #[error("receiver an error from input stream: {0}")]
    StreamingRequestError(#[from] Status),
    #[error("received an empty request")]
    EmptyRequest,
    #[error("received an invalid request: {reason}")]
    InvalidRequest { reason: String },
    #[error(transparent)]
    GameError(#[from] GameError),
}

impl<T> From<PoisonError<T>> for RpcError {
    fn from(value: PoisonError<T>) -> Self {
        Self::MutexPoison {
            description: value.to_string(),
        }
    }
}

impl<T> From<SendError<T>> for RpcError {
    fn from(value: SendError<T>) -> Self {
        Self::ChannelSendFailed {
            reason: value.to_string(),
        }
    }
}
