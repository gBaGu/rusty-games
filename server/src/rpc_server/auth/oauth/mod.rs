mod manager;
mod redirect;
mod worker;

pub use manager::{OAuth2Manager, OAuth2Meta, OAuth2Settings};
pub use redirect::RedirectListener;
pub use worker::GoogleApiWorker;

#[derive(Debug)]
pub struct UserInfo {
    name: String,
    email: String,
}

impl UserInfo {
    pub fn new(name: String, email: String) -> Self {
        Self { name, email }
    }
}
