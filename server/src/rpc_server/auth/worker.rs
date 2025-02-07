use std::future::{Future, IntoFuture};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::oauth::{OAuth2Meta, UserInfo};
use super::{AuthError, JWTClaims};
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
        db_connection: Arc<Mutex<db::Connection>>,
        secret: Vec<u8>,
        mut user_info_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            let key: Hmac<Sha256> = Hmac::new_from_slice(&secret).unwrap();
            while let Some((meta, user_info)) = user_info_receiver.recv().await {
                let mut db_lock = match db_connection.lock() {
                    Ok(guard) => guard,
                    Err(err) => {
                        meta.send_error(err.into());
                        continue;
                    }
                };
                let user_id = match db_lock.get_or_insert_user(user_info.name(), user_info.email())
                {
                    Some(user) => user.user_id,
                    None => {
                        meta.send_error(AuthError::Internal("todo: change to db error".into()));
                        continue;
                    }
                };
                drop(db_lock);
                println!("generating token for user: {}", user_id);

                let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                    meta.send_error(AuthError::TokenGenerationFailed(
                        "SystemTime before UNIX EPOCH!".into(),
                    ));
                    continue;
                };
                let claims = JWTClaims::new(user_id.try_into().unwrap(), now);
                let token_str = claims.sign_with_key(&key).unwrap();
                meta.send_token(token_str);
            }
        });
        Self(worker)
    }
}
