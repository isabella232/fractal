pub mod account;
pub mod contact;
pub mod server;

use matrix_sdk::identifiers::ServerName;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Display, Formatter};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Medium {
    Email,
    MsIsdn,
}

#[derive(Clone, Debug, Serialize)]
pub struct ThreePIDCredentials {
    pub client_secret: String,
    pub id_server: Box<ServerName>,
    pub sid: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AccessToken(String);

impl Display for AccessToken {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

// TODO: Remove this constructor when AccessToken is everywhere.
// It should not be manually created from the client
impl From<String> for AccessToken {
    fn from(token: String) -> Self {
        Self(token)
    }
}
