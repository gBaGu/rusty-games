use diesel::prelude::*;

use super::schema;

#[derive(Clone, Debug, Queryable, Selectable)]
#[diesel(table_name = schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub user_id: i64,
    pub name: String,
    pub email: String,
}

#[derive(Insertable)]
#[diesel(table_name = schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub email: &'a str,
}
