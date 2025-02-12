use std::sync::PoisonError;

#[derive(Debug, thiserror::Error)]
pub enum DbError {
    #[error(transparent)]
    Diesel(#[from] diesel::result::Error),
    #[error("failed to lock inner mutex: {0}")]
    MutexPoison(String),
}

impl<T> From<PoisonError<T>> for DbError {
    fn from(value: PoisonError<T>) -> Self {
        Self::MutexPoison(value.to_string())
    }
}
