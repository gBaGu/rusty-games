use std::str::FromStr;

use hmac::Hmac;
use sha2::Sha256;
use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::{Request, Status};

use super::token::{JWTClaims, JWTValidator};
use super::METADATA_KEY_USER_ID;

#[derive(Clone, Debug)]
pub struct ValidateJWT {
    validator: JWTValidator,
}

impl Interceptor for ValidateJWT {
    fn call(&mut self, mut request: Request<()>) -> Result<Request<()>, Status> {
        let Some(Ok(token)) = request.metadata().get("authorization").map(|v| v.to_str()) else {
            request.metadata_mut().remove(METADATA_KEY_USER_ID);
            return Ok(request);
        };
        let Some(token) = token.strip_prefix("Bearer ") else {
            return Err(Status::unauthenticated(
                "'Bearer ' prefix is missing in authorization header",
            ));
        };
        let claims: JWTClaims = self.validator.decode(token)?;
        let user_id = MetadataValue::from_str(claims.sub())
            .map_err(|_e| Status::internal("failed to convert user_id to header value"))?;
        request.metadata_mut().insert(METADATA_KEY_USER_ID, user_id);
        Ok(request)
    }
}

impl ValidateJWT {
    pub fn new(secret: Hmac<Sha256>) -> Self {
        Self {
            validator: JWTValidator::new(secret),
        }
    }
}
