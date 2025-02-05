mod error;
mod oauth;
mod rpc;

use error::AuthError;

pub use oauth::OAuth2Settings;
pub use rpc::AuthImpl;

type AuthResult<T> = Result<T, AuthError>;
