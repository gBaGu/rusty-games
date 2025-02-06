use std::future::{Future, IntoFuture};
use std::time::{SystemTime, UNIX_EPOCH};

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::oauth::{OAuth2Meta, UserInfo};
use super::{AuthError, JWTClaims};

const JWT_LIFETIME_SECS: u64 = 60 * 60;

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
        secret: Vec<u8>,
        mut user_info_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            let key: Hmac<Sha256> = Hmac::new_from_slice(&secret).unwrap();
            while let Some((meta, user_info)) = user_info_receiver.recv().await {
                // todo: save user_info to database and get user_id

                let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
                    meta.send_error(AuthError::TokenGenerationFailed(
                        "SystemTime before UNIX EPOCH!".into(),
                    ));
                    continue;
                };
                let now = now.as_secs();
                let claims = JWTClaims::new(0, now, now + JWT_LIFETIME_SECS);
                let token_str = claims.sign_with_key(&key).unwrap();
                meta.send_token(token_str);
            }
        });
        Self(worker)
    }
}
