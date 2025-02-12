use std::collections::hash_map::{Entry as HashMapEntry, HashMap};
use std::sync::Mutex;

use oauth2::basic::{BasicClient, BasicTokenResponse};
use oauth2::{reqwest, url};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::rpc_server::auth::{AuthError, AuthResult};

type OAuth2Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>;

/// Settings required for OAuth2.0 integration.
#[derive(Debug, Deserialize)]
pub struct OAuth2Settings {
    client_id: ClientId,
    client_secret: ClientSecret,
    auth_url: AuthUrl,
    token_url: TokenUrl,
    redirect_url: RedirectUrl,
    revocation_url: RevocationUrl,
    access_token_scopes: Vec<Scope>,
}

impl OAuth2Settings {
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

    pub fn redirect_url(&self) -> &RedirectUrl {
        &self.redirect_url
    }
}

/// Metadata that is used to exchange authorization code for access token and send it
/// over the channel for processing.
#[derive(Debug)]
pub struct OAuth2Meta {
    pkce_verifier: PkceCodeVerifier,
    jwt_sender: oneshot::Sender<AuthResult<String>>,
}

impl OAuth2Meta {
    pub fn new(
        pkce_verifier: PkceCodeVerifier,
        jwt_sender: oneshot::Sender<AuthResult<String>>,
    ) -> Self {
        Self {
            pkce_verifier,
            jwt_sender,
        }
    }

    pub fn send_error(self, err: AuthError) {
        if let Err(err) = self.jwt_sender.send(Err(err)) {
            // SAFETY: unwrap_err here is safe because we were trying to send Err() value
            println!("failed to send error back to rpc: {}", err.unwrap_err());
        }
    }

    pub fn send_token(self, token: String) {
        if let Err(_) = self.jwt_sender.send(Ok(token)) {
            println!("failed to send jwt token back to rpc");
        }
    }

    pub fn pkce_verifier(&self) -> &PkceCodeVerifier {
        &self.pkce_verifier
    }
}

/// Store OAuth2.0 metadata and provide interface to interact with OAuth2.0 api.
pub struct OAuth2Manager {
    auth_map: Mutex<HashMap<CsrfToken, OAuth2Meta>>,
    client: OAuth2Client,
    access_token_scopes: Vec<Scope>,
}

impl OAuth2Manager {
    pub fn new(settings: OAuth2Settings) -> Self {
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

    /// Exchange authorization code for an access token.
    pub async fn exchange_code(
        &self,
        code: AuthorizationCode,
        pkce_verifier: PkceCodeVerifier,
    ) -> AuthResult<BasicTokenResponse> {
        let http_client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|err| AuthError::ExchangeAuthCodeFailed(err.to_string()))?;
        self.client
            .exchange_code(code)
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client)
            .await
            .map_err(|err| AuthError::ExchangeAuthCodeFailed(err.to_string()))
    }

    /// Generate url to trigger OAuth2.0 flow.
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

    /// Insert metadata into inner map.
    pub fn insert(&self, key: CsrfToken, value: OAuth2Meta) -> AuthResult<()> {
        let mut guard = self.auth_map.lock()?;
        match guard.entry(key) {
            HashMapEntry::Vacant(e) => {
                e.insert(value);
            }
            HashMapEntry::Occupied(_) => Err(AuthError::DuplicateAuthMeta)?,
        }
        Ok(())
    }

    /// Remove metadata from inner map, return removed value.
    pub fn remove(&self, key: &CsrfToken) -> AuthResult<Option<OAuth2Meta>> {
        let mut guard = self.auth_map.lock()?;
        Ok(guard.remove(key))
    }
}
