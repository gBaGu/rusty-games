use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::rpc_server::auth::error::AuthError;
use oauth2::basic::BasicClient;
use oauth2::url;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenUrl,
};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::oneshot;
use tokio::task::JoinError;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;
use tokio_util::sync::CancellationToken;

use super::AuthResult;

type OAuth2Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>;

pub struct ClientSettings {
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    redirect_url: RedirectUrl,
    revocation_url: RevocationUrl,
    access_token_scopes: Vec<Scope>,
}

impl ClientSettings {
    pub fn new(
        client_id: impl Into<String>,
        client_secret: impl Into<String>,
        auth_url: impl Into<String>,
        token_url: impl Into<String>,
        redirect_url: impl Into<String>,
        revocation_url: impl Into<String>,
        access_token_scopes: impl IntoIterator<Item = impl Into<String>>,
    ) -> Result<Self, url::ParseError> {
        Ok(Self {
            client_id: ClientId::new(client_id.into()),
            client_secret: ClientSecret::new(client_secret.into()),
            auth_url: AuthUrl::new(auth_url.into())?,
            token_url: TokenUrl::new(token_url.into())?,
            redirect_url: RedirectUrl::new(redirect_url.into())?,
            revocation_url: RevocationUrl::new(revocation_url.into())?,
            access_token_scopes: access_token_scopes
                .into_iter()
                .map(Into::<String>::into)
                .map(Scope::new)
                .collect(),
        })
    }
}

#[derive(Debug)]
pub struct OAuth2Meta {
    started_at: Instant,
    pkce_verifier: PkceCodeVerifier,
    token_sender: oneshot::Sender<AuthResult<String>>,
}

impl OAuth2Meta {
    pub fn new(
        pkce_verifier: PkceCodeVerifier,
        token_sender: oneshot::Sender<AuthResult<String>>,
    ) -> Self {
        Self {
            started_at: Instant::now(),
            pkce_verifier,
            token_sender,
        }
    }
}

pub struct OAuth2Manager {
    auth_map: Mutex<HashMap<CsrfToken, OAuth2Meta>>,
    client: OAuth2Client,
    access_token_scopes: Vec<Scope>,
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
            access_token_scopes: settings.access_token_scopes,
        }
    }

    pub fn generate_auth_url(&self) -> (url::Url, CsrfToken, PkceCodeVerifier) {
        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
        let (auth_url, csrf_token) = self
            .client
            .authorize_url(CsrfToken::new_random)
            .add_scopes(self.access_token_scopes.clone())
            .set_pkce_challenge(pkce_challenge)
            .url();
        (auth_url, csrf_token, pkce_verifier)
    }

    pub fn insert(&self, key: CsrfToken, value: OAuth2Meta) -> AuthResult<()> {
        let mut guard = self.auth_map.lock()?;
        match guard.entry(key) {
            Entry::Vacant(e) => {
                e.insert(value);
            }
            Entry::Occupied(_) => Err(AuthError::DuplicateAuthMeta)?,
        }
        Ok(())
    }

    pub fn remove(&self, key: CsrfToken) -> AuthResult<()> {
        let mut guard = self.auth_map.lock()?;
        guard.remove(&key);
        Ok(())
    }

    pub fn take(&self, key: CsrfToken) -> AuthResult<OAuth2Meta> {
        let mut guard = self.auth_map.lock()?;
        match guard.entry(key) {
            Entry::Occupied(e) => Ok(e.remove()),
            Entry::Vacant(_) => Err(AuthError::MissingAuthMeta),
        }
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
