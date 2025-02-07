mod error;
mod oauth;
mod rpc;
mod worker;

use std::time::Duration;

use error::AuthError;
use serde::Serialize;

pub use oauth::OAuth2Settings;
pub use rpc::AuthImpl;

const JWT_LIFETIME_SECS: u64 = 60 * 60;

type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug, Serialize)]
pub struct JWTClaims {
    sub: u64,
    iat: u64,
    exp: u64,
}

impl JWTClaims {
    pub fn new(sub: u64, now: Duration) -> Self {
        Self {
            sub,
            iat: now.as_secs(),
            exp: now.as_secs() + JWT_LIFETIME_SECS,
        }
    }
}
