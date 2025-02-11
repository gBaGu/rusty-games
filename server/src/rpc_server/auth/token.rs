use std::ops::Deref;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use hmac::Hmac;
use jwt::{SignWithKey, VerifyWithKey};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use super::{AuthError, AuthResult};

const JWT_LIFETIME_SECS: u64 = 60 * 60;
const SYSTEM_TIME_ERROR_MESSAGE: &str = "SystemTime before UNIX EPOCH";

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

    pub fn sub(&self) -> &String {
        &self.sub
    }

    pub fn is_fresh(&self, now: Duration) -> bool {
        (self.iat..self.exp).contains(&now.as_secs())
    }
}

#[derive(Clone, Debug)]
pub struct JWTValidator(Hmac<Sha256>);

impl Deref for JWTValidator {
    type Target = Hmac<Sha256>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl JWTValidator {
    pub fn new(secret: Hmac<Sha256>) -> Self {
        Self(secret)
    }

    pub fn encode_from_sub(&self, sub: impl Into<String>) -> AuthResult<String> {
        let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return Err(AuthError::TokenGenerationFailed(
                SYSTEM_TIME_ERROR_MESSAGE.into(),
            ));
        };
        let claims = JWTClaims::new(sub.into(), now);
        claims
            .sign_with_key(self.deref())
            .map_err(|err| AuthError::TokenGenerationFailed(format!("unable to sign: {}", err)))
    }

    pub fn decode(&self, token: &str) -> AuthResult<JWTClaims> {
        let claims: JWTClaims = token
            .verify_with_key(self.deref())
            .map_err(|err| AuthError::TokenValidationFailed(err.to_string()))?;
        let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
            return Err(AuthError::TokenValidationFailed(
                SYSTEM_TIME_ERROR_MESSAGE.into(),
            ));
        };
        if !claims.is_fresh(now) {
            return Err(AuthError::TokenValidationFailed("token is expired".into()));
        }
        Ok(claims)
    }
}
