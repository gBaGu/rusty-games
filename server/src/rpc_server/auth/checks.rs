use tonic::metadata::MetadataMap;

use super::{AuthError, AuthResult, METADATA_KEY_USER_ID};
use crate::rpc_server::UserId;

/// Credentials check strategy.  
/// `Single`: provided credentials must match contained id;  
/// `OneOf`: provided credentials must match one of contained ids.
#[derive(Debug)]
pub enum Check<'a> {
    Single(UserId),
    OneOf(&'a [UserId]),
}

impl Check<'_> {
    /// Perform the check against `user_id`.
    pub fn matches(&self, user_id: UserId) -> bool {
        match self {
            Check::Single(id) => *id == user_id,
            Check::OneOf(ids) => ids.iter().find(|id| **id == user_id).is_some(),
        }
    }
}

/// Retrieve `METADATA_KEY_USER_ID` value from request metadata and perform the [`Check`] on it.
pub fn check_credentials(request_metadata: &MetadataMap, check: Check) -> AuthResult<()> {
    let user_id_value = request_metadata
        .get(METADATA_KEY_USER_ID)
        .ok_or(AuthError::MissingCredentials)?;
    let user_id_str = user_id_value
        .to_str()
        .map_err(|err| AuthError::InvalidCredentials(err.to_string()))?;
    let user_id = user_id_str
        .parse::<UserId>()
        .map_err(|err| AuthError::InvalidCredentials(err.to_string()))?;
    if !check.matches(user_id) {
        return Err(AuthError::wrong_credentials(
            format!("{:?}", check),
            user_id,
        ));
    }
    println!("credentials OK");
    Ok(())
}
