use std::future::{Future, IntoFuture};
use std::sync::Arc;

use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::oauth::{OAuth2Meta, UserInfo};
use super::token::JWTValidator;
use super::AuthError;
use crate::db;

pub struct LogInWorker(JoinHandle<()>);

impl IntoFuture for LogInWorker {
    type Output = <JoinHandle<()> as Future>::Output;
    type IntoFuture = JoinHandle<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.0.into_future()
    }
}

impl LogInWorker {
    pub fn new(
        db_connection: Arc<db::Connection>,
        secret: Vec<u8>,
        mut user_info_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            let jwt_validator = JWTValidator::new(secret);
            while let Some((meta, user_info)) = user_info_receiver.recv().await {
                let user_id =
                    match db_connection.get_or_insert_user(user_info.name(), user_info.email()) {
                        Ok(user) => user.user_id,
                        Err(err) => {
                            meta.send_error(AuthError::Db(err.into()));
                            continue;
                        }
                    };
                println!("generating token for user: {}", user_id);
                let token = match jwt_validator.encode_from_sub(user_id.to_string()) {
                    Ok(token) => token,
                    Err(err) => {
                        meta.send_error(err);
                        continue;
                    }
                };
                meta.send_token(token);
            }
        });
        Self(worker)
    }
}
