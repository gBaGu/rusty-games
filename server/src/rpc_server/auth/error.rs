use std::sync::PoisonError;

use tokio::sync::oneshot;
use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<oneshot::error::RecvError> for AuthError {
    fn from(value: oneshot::error::RecvError) -> Self {
        Self::internal(value.to_string())
    }
}

impl<T> From<PoisonError<T>> for AuthError {
    fn from(value: PoisonError<T>) -> Self {
        Self::internal(value.to_string())
    }
}

impl AuthError {
    pub fn internal(reason: impl Into<String>) -> Self {
        Self::Internal(reason.into())
    }
}

impl From<AuthError> for Status {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::Internal(msg) => Status::internal(msg),
        }
    }
}
