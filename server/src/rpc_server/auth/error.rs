use std::sync::PoisonError;

use tokio::sync::oneshot;
use tonic::Status;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("trying to insert authentication meta for the same state")]
    DuplicateAuthMeta,
    #[error("requested authentication meta is missing")]
    MissingAuthMeta,
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
            AuthError::DuplicateAuthMeta | AuthError::MissingAuthMeta => {
                Status::internal(value.to_string())
            }
            AuthError::Internal(msg) => Status::internal(msg),
        }
    }
}
