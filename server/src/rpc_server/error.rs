use std::sync::PoisonError;

use tokio::sync::mpsc::error::SendError;
use tonic::Status;

use super::GameId;
use crate::core::{GameError, ProtobufError};

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
    #[error("failed to lock inner mutex: {reason}")]
    MutexPoison { reason: String },
    #[error("invalid turn data: {source}")]
    TurnDataConversion {
        #[from]
        source: ProtobufError,
    },
    #[error("failed to send data over channel: {reason}")]
    ChannelSendFailed { reason: String },
    #[error("failed to read from input stream: {0}")]
    StreamingRequestReadFailed(#[from] Status),
    #[error("received an empty request")]
    EmptyRequest,
    #[error("unexpected request: expected {expected}, found: {found}")]
    UnexpectedRequest { expected: String, found: String },
    #[error("`{0}` is missing from request")]
    RequestDataMissing(String),
    #[error("worker is not running")]
    WorkerDown,
    #[error(transparent)]
    GameError(#[from] GameError),
}

impl<T> From<PoisonError<T>> for RpcError {
    fn from(value: PoisonError<T>) -> Self {
        Self::MutexPoison {
            reason: value.to_string(),
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

impl From<RpcError> for Status {
    fn from(value: RpcError) -> Self {
        match value {
            RpcError::DeleteActiveGameFailed => Status::failed_precondition(value.to_string()),
            RpcError::DuplicateGame => Status::already_exists(value.to_string()),
            RpcError::InvalidGameType => Status::invalid_argument(value.to_string()),
            RpcError::NoSuchGame { .. } => Status::not_found(value.to_string()),
            RpcError::ForeignGame => Status::permission_denied(value.to_string()),
            RpcError::StreamingRequestReadFailed(status) => status,
            RpcError::EmptyRequest => Status::invalid_argument(value.to_string()),
            RpcError::RequestDataMissing(_) => Status::invalid_argument(value.to_string()),
            RpcError::UnexpectedRequest { .. } => Status::failed_precondition(value.to_string()),
            RpcError::Internal { .. }
            | RpcError::MutexPoison { .. }
            | RpcError::TurnDataConversion { .. }
            | RpcError::ChannelSendFailed { .. }
            | RpcError::WorkerDown
            | RpcError::GameError(_) => Status::internal(value.to_string()),
        }
    }
}

impl RpcError {
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::Internal {
            reason: reason.into(),
        }
    }

    pub fn unexpected_request(expected: impl Into<String>, found: impl Into<String>) -> Self {
        Self::UnexpectedRequest {
            expected: expected.into(),
            found: found.into(),
        }
    }
}
