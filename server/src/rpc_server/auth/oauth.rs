use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use oauth2::basic::BasicClient;
use oauth2::url;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeVerifier,
    RedirectUrl, RevocationUrl, TokenUrl,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::task::JoinError;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use crate::rpc_server::rpc::RpcInnerResult;

type OAuth2Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>;

pub struct ClientSettings {
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    redirect_url: RedirectUrl,
    revocation_url: RevocationUrl,
}

impl ClientSettings {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        redirect_url: impl Into<String>,
        revocation_url: impl Into<String>,
    ) -> Result<Self, url::ParseError> {
        Ok(Self {
            client_id: ClientId::new(client_id.into()),
            client_secret: ClientSecret::new(client_secret.into()),
            auth_url: AuthUrl::new(auth_url.into())?,
            token_url: TokenUrl::new(token_url.into())?,
            redirect_url: RedirectUrl::new(redirect_url.into())?,
            revocation_url: RevocationUrl::new(revocation_url.into())?,
        })
    }
}

#[derive(Debug)]
pub struct OAuth2Meta {
    started_at: Instant,
    pkce_verifier: PkceCodeVerifier,
    // token_channel
}

pub struct OAuth2Manager {
    auth_map: Mutex<HashMap<CsrfToken, OAuth2Meta>>,
    client: OAuth2Client,
}

impl OAuth2Manager {
    pub fn new(settings: ClientSettings) -> Self {
        let client = BasicClient::new(settings.client_id)
            .set_client_secret(settings.client_secret)
            .set_auth_uri(settings.auth_url)
            .set_token_uri(settings.token_url)
            .set_redirect_uri(settings.redirect_url)
            .set_revocation_url(settings.revocation_url);
        Self {
            auth_map: Default::default(),
            client,
        }
    }

    pub fn insert(&self, key: CsrfToken, value: OAuth2Meta) -> RpcInnerResult<()> {
        let mut guard = self.auth_map.lock()?;
        todo!()
    }

    pub fn take(&self, key: CsrfToken) -> RpcInnerResult<OAuth2Meta> {
        let mut guard = self.auth_map.lock()?;
        todo!()
    }
}

pub struct RedirectListener {
    addr: SocketAddr,
    auth_manager: Arc<OAuth2Manager>,
}

impl RedirectListener {
    pub fn new(addr: SocketAddr, auth_manager: Arc<OAuth2Manager>) -> Self {
        Self { addr, auth_manager }
    }

    pub async fn start(&mut self, ct: CancellationToken) -> Result<(), JoinError> {
        let addr = self.addr;
        let auth_manager = self.auth_manager.clone();
        tokio::spawn(async move {
            let listener = match TcpListener::bind(addr).await {
                Ok(listener) => listener,
                Err(err) => {
                    println!("redirect listener: failed to bind to {}: {}", addr, err);
                    return;
                }
            };
            let mut listener = TcpListenerStream::new(listener);
            loop {
                tokio::select! {
                    biased;
                    _ = ct.cancelled() => {
                        println!("redirect listener: cancelled");
                        break;
                    }
                    next = listener.next() => {
                        let Some(stream) = next else {
                            break;
                        };
                        match stream {
                            Ok(stream) => {
                                println!("redirect listener: received connection");
                                tokio::spawn(handle_connection(stream, auth_manager.clone()));
                            }
                            Err(err) => {
                                println!("redirect listener: connection failed");
                            }
                        }
                    }
                }
            }
            println!("redirect listener: finished");
        })
        .await
    }
}

async fn handle_connection(stream: TcpStream, auth_manager: Arc<OAuth2Manager>) {}
