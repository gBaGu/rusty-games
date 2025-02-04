use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use super::oauth::{OAuth2Manager, OAuth2Meta};
use super::AuthError;
use crate::proto;
use crate::rpc_server::RpcResult;

pub struct AuthImpl {
    auth_manager: Arc<OAuth2Manager>,
}

impl AuthImpl {
    pub fn new(auth_manager: Arc<OAuth2Manager>) -> Self {
        Self { auth_manager }
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
        let auth_manager_cloned = self.auth_manager.clone();
        let stream = tokio_stream::once(Ok(proto::LogInReply::auth_link(auth_url.to_string())))
            .chain(async_stream::try_stream! {
                let res = token_receiver.await;
                if res.is_err() {
                    auth_manager_cloned.remove(csrf_token)?;
                }
                let token = res.map_err(AuthError::from)??;
                yield proto::LogInReply::token(token)
            });
        Ok(Response::new(Box::pin(stream)))
    }
}
