mod connection;
mod error;
mod models;
mod schema;

pub use connection::Connection;
pub use error::DbError;

type DbResult<T> = Result<T, DbError>;

pub trait DbBasic: Send + Sync + 'static {
    /// If `users` table has a record with requested `email` return it. Otherwise,
    /// create a new record with provided `name` and `email` and return inserted user.
    fn get_or_insert_user(&self, name: &str, email: &str) -> DbResult<models::User>;
}
