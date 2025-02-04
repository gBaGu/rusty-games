use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use super::oauth::{OAuth2Manager, OAuth2Meta};
use super::{AuthError, AuthResult};
use crate::proto;
use crate::rpc_server::RpcResult;

fn make_log_in_reply_stream(
    auth_url: String,
    token_receiver: oneshot::Receiver<AuthResult<String>>,
) -> impl Stream<Item = Result<proto::LogInReply, Status>> {
    tokio_stream::once(Ok(proto::LogInReply {
        reply: Some(proto::log_in_reply::Reply::Link(auth_url)),
    }))
    .chain(async_stream::try_stream! {
        let token = token_receiver.await.map_err(AuthError::from)??;
        yield proto::LogInReply {
            reply: Some(proto::log_in_reply::Reply::Token(token)),
        }
    })
}

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
        let (s, r) = oneshot::channel();
        self.auth_manager
            .insert(csrf_token, OAuth2Meta::new(pkce_verifier, s))?;
        let stream = make_log_in_reply_stream(auth_url.to_string(), r);
        Ok(Response::new(Box::pin(stream)))
    }
}
