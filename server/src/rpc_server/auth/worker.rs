use std::future::{Future, IntoFuture};

use hmac::{Hmac, Mac};
use jwt::SignWithKey;
use sha2::Sha256;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::JWTClaims;
use super::oauth::{OAuth2Meta, UserInfo};

pub struct LogInWorker(JoinHandle<()>);

impl IntoFuture for LogInWorker {
    type Output = <JoinHandle<()> as Future>::Output;
    type IntoFuture = JoinHandle<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.0.into_future()
    }
}

impl LogInWorker {
    pub fn new(mut user_info_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, UserInfo)>) -> Self {
        let worker = tokio::spawn(async move {
            while let Some((meta, user_info)) = user_info_receiver.recv().await {
                todo!()
            }
        });
        Self(worker)
    }
}
