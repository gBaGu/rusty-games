use std::future::{Future, IntoFuture};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::Hmac;
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
        db_connection: Arc<db::Connection>,
        secret: Hmac<Sha256>,
        mut user_info_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
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
                let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                    meta.send_error(AuthError::TokenGenerationFailed(
                        "SystemTime before UNIX EPOCH".into(),
                    ));
                    continue;
                };
                let claims = JWTClaims::new(user_id.to_string(), now);
                let token_str = match claims.sign_with_key(&secret) {
                    Ok(token) => token,
                    Err(err) => {
                        meta.send_error(AuthError::TokenGenerationFailed(format!(
                            "unable to sign: {}",
                            err
                        )));
                        continue;
                    }
                };
                meta.send_token(token_str);
            }
        });
        Self(worker)
    }
}
