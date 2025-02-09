mod checks;
mod error;
mod interceptor;
mod oauth;
mod rpc;
mod worker;

use std::time::Duration;

use serde::{Deserialize, Serialize};

pub use checks::{check_credentials, Check};
pub use error::AuthError;
pub use interceptor::ValidateJWT;
pub use oauth::OAuth2Settings;
pub use rpc::AuthImpl;

pub const METADATA_KEY_USER_ID: &str = "user-id";

const JWT_LIFETIME_SECS: u64 = 60 * 60;

type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug, Deserialize, Serialize)]
pub struct JWTClaims {
    sub: String,
    iat: u64,
    exp: u64,
}

impl JWTClaims {
    pub fn new(sub: String, now: Duration) -> Self {
        Self {
            sub,
            iat: now.as_secs(),
            exp: now.as_secs() + JWT_LIFETIME_SECS,
        }
    }

    pub fn is_fresh(&self, now: Duration) -> bool {
        (self.iat..self.exp).contains(&now.as_secs())
    }
}
