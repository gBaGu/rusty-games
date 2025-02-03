use std::pin::Pin;
use std::sync::Arc;

use tokio::sync::oneshot;
use tokio_stream::{Stream, StreamExt};
use tonic::{Request, Response, Status};

use super::oauth::{OAuth2Manager, OAuth2Meta};
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

    async fn log_in(&self, request: Request<proto::LogInRequest>) -> RpcResult<Self::LogInStream> {
        let (auth_url, csrf_token, pkce_verifier) = self.auth_manager.generate_auth_url();
        let (s, token_receiver) = oneshot::channel();
        self.auth_manager
            .insert(csrf_token, OAuth2Meta::new(pkce_verifier, s))?;
        let stream = tokio_stream::once(Ok(proto::LogInReply {
            reply: Some(proto::log_in_reply::Reply::Link(auth_url.to_string())),
        }))
        .chain(async_stream::stream! {
            match token_receiver.await {
                Ok(token) => yield Ok(proto::LogInReply {
                    reply: Some(proto::log_in_reply::Reply::Token(token)),
                }),
                Err(err) => yield Err(Status::internal(err.to_string())),
            }
        });
        Ok(Response::new(Box::pin(stream)))
    }
}
