use std::future::{Future, IntoFuture};

use oauth2::basic::BasicTokenResponse;
use oauth2::{reqwest, TokenResponse};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::{OAuth2Meta, UserInfo};
use crate::rpc_server::auth::AuthError;

const GOOGLE_USERINFO_API: &str = "https://www.googleapis.com/oauth2/v3/userinfo";

#[derive(Debug, Deserialize)]
struct GoogleUserInfo {
    name: String,
    email: String,
}

impl From<GoogleUserInfo> for UserInfo {
    fn from(value: GoogleUserInfo) -> Self {
        UserInfo::new(value.name, value.email)
    }
}

pub struct GoogleApiWorker(JoinHandle<()>);

impl IntoFuture for GoogleApiWorker {
    type Output = <JoinHandle<()> as Future>::Output;
    type IntoFuture = JoinHandle<()>;

    fn into_future(self) -> Self::IntoFuture {
        self.0.into_future()
    }
}

impl GoogleApiWorker {
    pub fn new(
        mut token_receiver: mpsc::UnboundedReceiver<(OAuth2Meta, BasicTokenResponse)>,
        user_info_sender: mpsc::UnboundedSender<(OAuth2Meta, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            let client = reqwest::Client::default();
            while let Some((meta, token_response)) = token_receiver.recv().await {
                let response = match client
                    .get(GOOGLE_USERINFO_API)
                    .bearer_auth(token_response.access_token().secret())
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        meta.send_error(AuthError::GoogleApiFetchFailed(err.to_string()));
                        continue;
                    }
                };
                let text = match response.text().await {
                    Ok(text) => text,
                    Err(err) => {
                        meta.send_error(AuthError::GoogleApiFetchFailed(err.to_string()));
                        continue;
                    }
                };
                let user_info: GoogleUserInfo = match serde_json::from_str(&text) {
                    Ok(info) => info,
                    Err(err) => {
                        meta.send_error(AuthError::GoogleApiFetchFailed(err.to_string()));
                        continue;
                    }
                };
                println!(
                    "got user info: name={}, email={}",
                    user_info.name, user_info.email
                );
                if let Err(err) = user_info_sender.send((meta, user_info.into())) {
                    let msg = err.to_string();
                    err.0 .0.send_error(AuthError::GoogleApiFetchFailed(msg));
                }
            }
        });
        Self(worker)
    }
}
