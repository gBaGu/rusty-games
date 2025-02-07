use std::env;

use diesel::prelude::*;
use diesel::Connection as _;

use super::models::*;
use super::schema::users;

pub struct Connection {
    inner: PgConnection,
}

impl Connection {
    pub fn new() -> Self {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
        let conn = PgConnection::establish(&database_url)
            .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));
        Self { inner: conn }
    }

    pub fn get_or_insert_user(&mut self, name: &str, email: &str) -> Option<User> {
        let results = users::table
            .filter(users::email.eq(email))
            .select(User::as_select())
            .load(&mut self.inner)
            .ok()?;
        if results.is_empty() {
            println!("inserting new user: {}", name);
            let new_user = NewUser { name, email };
            return diesel::insert_into(users::table)
                .values(&new_user)
                .returning(User::as_returning())
                .get_result(&mut self.inner)
                .ok();
        }
        let [user]: [User; 1] = results.try_into().ok()?;
        Some(user)
    }

    pub fn get_user_by_email(&mut self, email: &str) -> Option<User> {
        let results = users::table
            .filter(users::email.eq(email))
            .select(User::as_select())
            .load(&mut self.inner)
            .expect("Error loading posts");
        let [user]: [User; 1] = match results.try_into() {
            Ok(val) => val,
            Err(_) => return None,
        };
        Some(user)
    }

    pub fn create_user(&mut self, name: &str, email: &str) -> Option<User> {
        let new_user = NewUser { name, email };
        diesel::insert_into(users::table)
            .values(&new_user)
            .returning(User::as_returning())
            .get_result(&mut self.inner)
            .ok()
    }
}
