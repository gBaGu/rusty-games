use std::ops::Deref;
use std::time::{SystemTime, UNIX_EPOCH};

use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use super::AuthResult;

const JWT_LIFETIME_SECS: u64 = 60 * 60;

/// JWT payload
#[derive(Debug, Deserialize, Serialize)]
pub struct JWTClaims {
    sub: String,
    iat: u64,
    exp: u64,
}

impl JWTClaims {
    pub fn new(sub: String, iat: u64) -> Self {
        Self {
            sub,
            iat,
            exp: iat + JWT_LIFETIME_SECS,
        }
    }

    pub fn sub(&self) -> &String {
        &self.sub
    }
}

/// Validator that can sign/decode jwt using secret.
#[derive(Clone, Debug)]
pub struct JWTValidator(Vec<u8>);

impl Deref for JWTValidator {
    type Target = Vec<u8>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl JWTValidator {
    pub fn new(secret: Vec<u8>) -> Self {
        Self(secret)
    }

    /// Create [`JWTClaims`] from `sub` value, sign and encode the token.
    pub fn encode_from_sub(&self, sub: impl Into<String>) -> AuthResult<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let claims = JWTClaims::new(sub.into(), now.as_secs());
        let key = EncodingKey::from_secret(self.as_slice());
        Ok(jsonwebtoken::encode(&Header::default(), &claims, &key)?)
    }

    /// Decode and validate jwt, return decoded claims.
    pub fn decode(&self, token: &str) -> AuthResult<JWTClaims> {
        let key = DecodingKey::from_secret(self.as_slice());
        let mut validation = Validation::new(Default::default());
        validation.validate_exp = true;
        let decoded = jsonwebtoken::decode(token, &key, &validation)?;
        Ok(decoded.claims)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn encode_decode() {
        let validator = JWTValidator::new(b"test".to_vec());
        let test_sub = "0";
        let encoded = validator.encode_from_sub("0").unwrap();
        let decoded = validator.decode(&encoded).unwrap();
        assert_eq!(decoded.sub, test_sub);
    }
}
