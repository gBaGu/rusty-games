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

pub struct AuthImpl {
    auth_manager: Arc<OAuth2Manager>,
    db_connection: Arc<db::Connection>,
}

impl AuthImpl {
    pub fn new(oauth2_settings: OAuth2Settings, db_connection: db::Connection) -> Self {
        Self {
            auth_manager: Arc::new(OAuth2Manager::new(oauth2_settings)),
            db_connection: Arc::new(db_connection),
        }
    }

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
impl proto::auth_server::Auth for AuthImpl {
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
