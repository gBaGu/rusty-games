use std::str::FromStr;

use tonic::metadata::MetadataValue;
use tonic::service::Interceptor;
use tonic::{Request, Status};

use super::token::{JWTClaims, JWTValidator};
use super::METADATA_KEY_USER_ID;

/// Interceptor that validates JWT and inserts `sub` value from its claims into request metadata.
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
            return Err(Status::unauthenticated("'Bearer ' prefix is missing"));
        };
        let claims: JWTClaims = self.validator.decode(token)?;
        let user_id = MetadataValue::from_str(claims.sub())
            .map_err(|_e| Status::internal("failed to convert user_id to header value"))?;
        request.metadata_mut().insert(METADATA_KEY_USER_ID, user_id);
        Ok(request)
    }
}

impl ValidateJWT {
    pub fn new(secret: Vec<u8>) -> Self {
        Self {
            validator: JWTValidator::new(secret),
        }
    }
}

#[cfg(test)]
mod test {
    use tonic::Code;

    use super::*;

    #[test]
    fn validate_jwt_call_error() {
        let mut validate_jwt = ValidateJWT::new(b"test".to_vec());

        // METADATA_KEY_USER_ID is removed if authorization is missing
        let mut req = Request::new(());
        req.metadata_mut()
            .insert(METADATA_KEY_USER_ID, MetadataValue::from_str("").unwrap());
        req = validate_jwt.call(req).unwrap();
        assert!(!req.metadata().contains_key(METADATA_KEY_USER_ID));

        // prefix doesn't match 'Bearer '
        req = Request::new(());
        req.metadata_mut()
            .insert("authorization", MetadataValue::from_str("bearer ").unwrap());
        let status = validate_jwt.call(req).unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert_eq!(status.message(), "'Bearer ' prefix is missing");

        // empty token
        req = Request::new(());
        req.metadata_mut()
            .insert("authorization", MetadataValue::from_str("Bearer ").unwrap());
        let status = validate_jwt.call(req).unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert!(status.message().starts_with("jwt error: "));

        // token missing signature
        req = Request::new(());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_str(
                "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.",
            )
            .unwrap(),
        );
        let status = validate_jwt.call(req).unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert!(status.message().starts_with("jwt error: "));

        // invalid signature
        req = Request::new(());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_str(
                "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c",
            )
                .unwrap(),
        );
        let status = validate_jwt.call(req).unwrap_err();
        assert_eq!(status.code(), Code::Unauthenticated);
        assert!(status.message().starts_with("jwt error: "));
    }

    #[test]
    fn validate_jwt_call_success() {
        let mut validate_jwt = ValidateJWT::new(b"test".to_vec());
        let test_sub = "0";
        let token = validate_jwt.validator.encode_from_sub(test_sub).unwrap();

        // just a Bearer header
        let mut req = Request::new(());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        req = validate_jwt.call(req).unwrap();
        assert_eq!(
            req.metadata()
                .get(METADATA_KEY_USER_ID)
                .unwrap()
                .to_str()
                .unwrap(),
            test_sub,
        );

        // the same but with inserted METADATA_KEY_USER_ID value that doesn't match sub from the token
        let mut req = Request::new(());
        req.metadata_mut()
            .insert(METADATA_KEY_USER_ID, MetadataValue::from_str("1").unwrap());
        req.metadata_mut().insert(
            "authorization",
            MetadataValue::from_str(&format!("Bearer {}", token)).unwrap(),
        );
        req = validate_jwt.call(req).unwrap();
        assert_eq!(
            req.metadata().get_all(METADATA_KEY_USER_ID).iter().count(),
            1
        );
        assert_eq!(
            req.metadata()
                .get(METADATA_KEY_USER_ID)
                .unwrap()
                .to_str()
                .unwrap(),
            test_sub,
        );
    }
}
