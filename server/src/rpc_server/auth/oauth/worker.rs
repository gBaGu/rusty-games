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

fn send_google_api_error_to_rpc(meta: OAuth2Meta, msg: String) {
    if let Err(err) = meta
        .into_jwt_sender()
        .send(Err(AuthError::GoogleApiFetchFailed(msg)))
    {
        // SAFETY: unwrap_err here is safe because we were trying to send Err() value
        println!("GoogleApiWorker: failed to send error back to rpc: {}", err.unwrap_err());
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
                        send_google_api_error_to_rpc(meta, err.to_string());
                        continue;
                    }
                };
                let text = match response.text().await {
                    Ok(text) => text,
                    Err(err) => {
                        send_google_api_error_to_rpc(meta, err.to_string());
                        continue;
                    }
                };
                let user_info: GoogleUserInfo = match serde_json::from_str(&text) {
                    Ok(info) => info,
                    Err(err) => {
                        send_google_api_error_to_rpc(meta, err.to_string());
                        continue;
                    }
                };
                if let Err(err) = user_info_sender.send((meta, user_info.into())) {
                    let msg = err.to_string();
                    send_google_api_error_to_rpc(err.0 .0, msg);
                }
            }
        });
        Self(worker)
    }
}
