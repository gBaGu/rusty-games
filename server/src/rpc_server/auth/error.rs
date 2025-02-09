use std::sync::PoisonError;

use tokio::sync::oneshot;
use tonic::Status;

use crate::db::DbError;
use crate::rpc_server::UserId;

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("trying to insert authentication meta for the same state")]
    DuplicateAuthMeta,
    #[error("credentials data is missing")]
    MissingCredentials,
    #[error("invalid credentials: {0}")]
    InvalidCredentials(String),
    #[error("expected credentials: {}, found: {}", .expected, .found)]
    WrongCredentials { expected: String, found: UserId },
    #[error("failed to generate jwt token: {0}")]
    TokenGenerationFailed(String),
    #[error("failed to get data from google api: {0}")]
    GoogleApiFetchFailed(String),
    #[error("database query failed: {0}")]
    Db(#[from] DbError),
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

    pub fn wrong_credentials(expected: impl Into<String>, found: UserId) -> Self {
        Self::WrongCredentials {
            expected: expected.into(),
            found,
        }
    }
}

impl From<AuthError> for Status {
    fn from(value: AuthError) -> Self {
        match value {
            AuthError::InvalidCredentials(_) | AuthError::MissingCredentials => {
                Status::unauthenticated(value.to_string())
            }
            AuthError::WrongCredentials { .. } => Status::permission_denied(value.to_string()),
            AuthError::DuplicateAuthMeta
            | AuthError::TokenGenerationFailed(_)
            | AuthError::GoogleApiFetchFailed(_)
            | AuthError::Db(_) => Status::internal(value.to_string()),
            AuthError::Internal(msg) => Status::internal(msg),
        }
    }
}
