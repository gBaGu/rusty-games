mod error;
mod oauth;
mod rpc;

use error::AuthError;

type AuthResult<T> = Result<T, AuthError>;
