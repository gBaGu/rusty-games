use std::collections::hash_map::{Entry as HashMapEntry, HashMap};
use std::sync::Mutex;

use oauth2::basic::BasicClient;
use oauth2::url;
use oauth2::{
    AuthUrl, ClientId, ClientSecret, CsrfToken, EndpointNotSet, EndpointSet, PkceCodeChallenge,
    PkceCodeVerifier, RedirectUrl, RevocationUrl, Scope, TokenUrl,
};
use serde::Deserialize;
use tokio::sync::oneshot;

use crate::rpc_server::auth::{AuthError, AuthResult};

type OAuth2Client =
    BasicClient<EndpointSet, EndpointNotSet, EndpointNotSet, EndpointSet, EndpointSet>;

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
}

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
}

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

    pub fn client(&self) -> &OAuth2Client {
        &self.client
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

    pub fn get_pkce_verifier(&self, key: &CsrfToken) -> AuthResult<PkceCodeVerifier> {
        let guard = self.auth_map.lock()?;
        guard
            .get(key)
            .map(|meta| PkceCodeVerifier::new(meta.pkce_verifier.secret().clone()))
            .ok_or(AuthError::MissingAuthMeta)
    }

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

    pub fn remove(&self, key: CsrfToken) -> AuthResult<()> {
        let mut guard = self.auth_map.lock()?;
        guard.remove(&key);
        Ok(())
    }
}
