mod error;
mod oauth;
mod rpc;
mod worker;

use error::AuthError;
use serde::Serialize;

pub use oauth::OAuth2Settings;
pub use rpc::AuthImpl;

type AuthResult<T> = Result<T, AuthError>;

#[derive(Debug, Serialize)]
pub struct JWTClaims {
    sub: u64,
    iat: u64,
    exp: u64,
}

impl JWTClaims {
    pub fn new(sub: u64, iat: u64, exp: u64) -> Self {
        Self { sub, iat, exp }
    }
}
