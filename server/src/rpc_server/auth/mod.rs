mod checks;
mod error;
mod interceptor;
mod oauth;
mod rpc;
mod token;
mod worker;

pub use checks::{check_credentials, Check};
pub use error::AuthError;
pub use interceptor::ValidateJWT;
pub use oauth::OAuth2Settings;
pub use rpc::AuthImpl;

pub const METADATA_KEY_USER_ID: &str = "user-id";

type AuthResult<T> = Result<T, AuthError>;
