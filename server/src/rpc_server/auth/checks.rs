use tonic::metadata::MetadataMap;

use super::{AuthError, AuthResult, METADATA_KEY_USER_ID};
use crate::rpc_server::UserId;

#[derive(Debug)]
pub enum Check<'a> {
    Single(UserId),
    OneOf(&'a [UserId]),
}

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
    let check_result = match check {
        Check::Single(id) => id == user_id,
        Check::OneOf(ids) => ids.iter().find(|id| **id == user_id).is_some(),
    };
    if !check_result {
        return Err(AuthError::wrong_credentials(
            format!("{:?}", check),
            user_id,
        ));
    }
    println!("credentials OK");
    Ok(())
}
