use std::future::Future;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinError;
use tokio_stream::{Stream, StreamExt};
use tokio_util::sync::CancellationToken;
use tonic::{Request, Response, Status};

use super::oauth::{GoogleApiWorker, OAuth2Manager, OAuth2Meta, OAuth2Settings, RedirectListener};
use super::{worker::LogInWorker, AuthError};
use crate::rpc_server::RpcResult;
use crate::{db, proto};

pub struct AuthImpl<DB> {
    auth_manager: Arc<OAuth2Manager>,
    db_connection: Arc<DB>,
}

impl<DB> AuthImpl<DB> {
    pub fn new(oauth2_settings: OAuth2Settings, db_connection: DB) -> Self {
        Self {
            auth_manager: Arc::new(OAuth2Manager::new(oauth2_settings)),
            db_connection: Arc::new(db_connection),
        }
    }
}

impl<DB: db::DbBasic> AuthImpl<DB> {
    /// Start workers and return the future waiting for them to complete.
    pub fn start(
        &mut self,
        jwt_secret: Vec<u8>,
        redirect_addr: SocketAddr,
        ct: CancellationToken,
    ) -> impl Future<Output = Result<(), JoinError>> {
        let google_token_channel = mpsc::unbounded_channel();
        let user_info_channel = mpsc::unbounded_channel();
        let redirect_listener = RedirectListener::new(
            redirect_addr,
            self.auth_manager.clone(),
            google_token_channel.0,
            ct,
        );
        let google_api_worker = GoogleApiWorker::new(google_token_channel.1, user_info_channel.0);
        let log_in_worker =
            LogInWorker::new(self.db_connection.clone(), jwt_secret, user_info_channel.1);
        async move {
            redirect_listener.await?;
            google_api_worker.await?;
            log_in_worker.await
        }
    }
}

#[tonic::async_trait]
impl<DB: db::DbBasic> proto::auth_server::Auth for AuthImpl<DB> {
    type LogInStream =
        Pin<Box<dyn Stream<Item = Result<proto::LogInReply, Status>> + Send + 'static>>;

    async fn log_in(&self, _request: Request<proto::LogInRequest>) -> RpcResult<Self::LogInStream> {
        let (auth_url, csrf_token, pkce_verifier) = self.auth_manager.generate_auth_url();
        let (s, token_receiver) = oneshot::channel();
        self.auth_manager
            .insert(csrf_token.clone(), OAuth2Meta::new(pkce_verifier, s))?;
        println!("log in url: {}", auth_url);
        let auth_manager_cloned = self.auth_manager.clone();
        let stream = tokio_stream::once(Ok(proto::LogInReply::auth_link(auth_url.to_string())))
            .chain(async_stream::try_stream! {
                let res = token_receiver.await;
                let _ = auth_manager_cloned.remove(&csrf_token)?;
                let token = res.map_err(AuthError::from)??;
                yield proto::LogInReply::token(token)
            });
        Ok(Response::new(Box::pin(stream)))
    }
}

#[cfg(test)]
mod test {
    use oauth2::CsrfToken;
    use regex::Regex;
    use tonic::Code;

    use super::*;
    use crate::proto::auth_server::Auth;

    const URL_PARAMS_RE: &str = r"\?response_type=code.*&state=([-_A-Za-z0-9]+)&code_challenge=.*";

    fn reply_unwrap_link(reply: proto::LogInReply) -> String {
        match reply.reply.unwrap() {
            proto::log_in_reply::Reply::Link(s) => s,
            _ => panic!("expected authentication link"),
        }
    }

    fn reply_unwrap_token(reply: proto::LogInReply) -> String {
        match reply.reply.unwrap() {
            proto::log_in_reply::Reply::Token(s) => s,
            _ => panic!("expected token"),
        }
    }

    #[tokio::test]
    async fn log_in_success() {
        let test_url = "http://localhost";
        let settings =
            OAuth2Settings::new("", "", test_url, test_url, test_url, test_url, [""]).unwrap();
        let auth = AuthImpl::new(settings, db::MockDbBasic::new());

        let request = Request::new(proto::LogInRequest {});
        let mut stream = auth.log_in(request).await.unwrap().into_inner();
        // receive authentication url
        let auth_url = reply_unwrap_link(stream.next().await.unwrap().unwrap());
        let re = Regex::new(&format!(r"{}/{}", test_url, URL_PARAMS_RE)).unwrap();
        // extract state parameter from url
        let (_, [state]) = re.captures(&auth_url).unwrap().extract();
        println!("state: {}", state);
        let state = CsrfToken::new(state.into());
        let meta = auth.auth_manager.remove(&state).unwrap().unwrap();
        // send the token back to rpc
        meta.send_token("some token".into());
        // receive the token
        let token = reply_unwrap_token(stream.next().await.unwrap().unwrap());
        assert_eq!(token, "some token".to_string());
    }

    #[tokio::test]
    async fn log_in_token_generation_error() {
        let test_url = "http://localhost";
        let settings =
            OAuth2Settings::new("", "", test_url, test_url, test_url, test_url, [""]).unwrap();
        let auth = AuthImpl::new(settings, db::MockDbBasic::new());

        let request = Request::new(proto::LogInRequest {});
        let mut stream = auth.log_in(request).await.unwrap().into_inner();
        // receive authentication url
        let auth_url = reply_unwrap_link(stream.next().await.unwrap().unwrap());
        let re = Regex::new(&format!(r"{}/{}", test_url, URL_PARAMS_RE)).unwrap();
        // extract state parameter from url
        let (_, [state]) = re.captures(&auth_url).unwrap().extract();
        println!("state: {}", state);
        let state = CsrfToken::new(state.into());
        let meta = auth.auth_manager.remove(&state).unwrap().unwrap();
        // send error back to rpc
        meta.send_error(AuthError::internal("some error"));
        let status = stream.next().await.unwrap().unwrap_err();
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), "some error");

        // the same as above but instead of sending error to rpc sender gets dropped unexpectedly
        let request = Request::new(proto::LogInRequest {});
        let mut stream = auth.log_in(request).await.unwrap().into_inner();
        // receive authentication url
        let auth_url = reply_unwrap_link(stream.next().await.unwrap().unwrap());
        let re = Regex::new(&format!(r"{}/{}", test_url, URL_PARAMS_RE)).unwrap();
        // extract state parameter from url
        let (_, [state]) = re.captures(&auth_url).unwrap().extract();
        println!("state: {}", state);
        let state = CsrfToken::new(state.into());
        let meta = auth.auth_manager.remove(&state).unwrap().unwrap();
        drop(meta);
        let status = stream.next().await.unwrap().unwrap_err();
        assert_eq!(status.code(), Code::Internal);
        assert_eq!(status.message(), "channel closed");
    }
}
