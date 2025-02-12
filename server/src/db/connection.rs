use std::sync::Mutex;

use diesel::prelude::*;
use diesel::Connection as _;

use super::models::*;
use super::schema::users;
use super::DbError;

type DbResult<T> = Result<T, DbError>;

/// Synchronized PostgreSQL connection.
pub struct Connection {
    inner: Mutex<PgConnection>,
}

impl Connection {
    pub fn new(database_url: &str) -> Self {
        let conn = PgConnection::establish(database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
        Self {
            inner: Mutex::new(conn),
        }
    }

    /// If `users` table has a record with requested `email` return it. Otherwise,
    /// create a new record with provided `name` and `email` and return inserted user.
    pub fn get_or_insert_user(&self, name: &str, email: &str) -> DbResult<User> {
        let mut guard = self.inner.lock()?;
        let results = get_user_by_email(&mut *guard, email)?;
        let user = match results.into_iter().next() {
            Some(user) => user,
            None => {
                println!("inserting new user: {}", name);
                create_user(&mut *guard, name, email)?
            }
        };
        Ok(user)
    }
}

/// Select from `users` table filtering by `email` field.
fn get_user_by_email(conn: &mut PgConnection, email: &str) -> QueryResult<Vec<User>> {
    users::table
        .filter(users::email.eq(email))
        .select(User::as_select())
        .load(conn)
}

/// Insert into `users` table.
fn create_user(conn: &mut PgConnection, name: &str, email: &str) -> QueryResult<User> {
    let new_user = NewUser { name, email };
    diesel::insert_into(users::table)
        .values(&new_user)
        .returning(User::as_returning())
        .get_result(conn)
}
