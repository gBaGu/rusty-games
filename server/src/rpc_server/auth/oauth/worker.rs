use std::future::{Future, IntoFuture};
use std::sync::Arc;

use oauth2::basic::BasicTokenResponse;
use oauth2::{reqwest, CsrfToken, TokenResponse};
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::{OAuth2Manager, UserInfo};
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

fn send_google_api_error_to_rpc(auth_manager: &OAuth2Manager, state: &CsrfToken, msg: String) {
    if let Err(err) =
        auth_manager.send_auth_result(state, Err(AuthError::GoogleApiFetchFailed(msg)))
    {
        println!(
            "GoogleApiWorker: failed to send result back to rpc: {}",
            err
        );
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
        auth_manager: Arc<OAuth2Manager>,
        mut token_receiver: mpsc::UnboundedReceiver<(CsrfToken, BasicTokenResponse)>,
        user_info_sender: mpsc::UnboundedSender<(CsrfToken, UserInfo)>,
    ) -> Self {
        let worker = tokio::spawn(async move {
            let client = reqwest::Client::default();
            while let Some((state, token_response)) = token_receiver.recv().await {
                let response = match client
                    .get(GOOGLE_USERINFO_API)
                    .bearer_auth(token_response.access_token().secret())
                    .send()
                    .await
                {
                    Ok(response) => response,
                    Err(err) => {
                        send_google_api_error_to_rpc(&auth_manager, &state, err.to_string());
                        continue;
                    }
                };
                let text = match response.text().await {
                    Ok(text) => text,
                    Err(err) => {
                        send_google_api_error_to_rpc(&auth_manager, &state, err.to_string());
                        continue;
                    }
                };
                let user_info: GoogleUserInfo = match serde_json::from_str(&text) {
                    Ok(info) => info,
                    Err(err) => {
                        send_google_api_error_to_rpc(&auth_manager, &state, err.to_string());
                        continue;
                    }
                };
                if let Err(err) = user_info_sender.send((state.clone(), user_info.into())) {
                    send_google_api_error_to_rpc(&auth_manager, &state, err.to_string());
                }
            }
        });
        Self(worker)
    }
}
