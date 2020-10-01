use super::Identifier;
use crate::api::r0::AccessToken;
use matrix_sdk::identifiers::DeviceId;
use matrix_sdk::identifiers::UserId;
use matrix_sdk::reqwest::Client;
use matrix_sdk::reqwest::Error;
use matrix_sdk::reqwest::Request;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Body {
    #[serde(flatten)]
    pub identifier: Identifier,
    #[serde(flatten)]
    pub auth: Auth,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub device_id: Option<Box<DeviceId>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_device_display_name: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "type")]
pub enum Auth {
    #[serde(rename = "m.login.password")]
    Password { password: String },
    #[allow(dead_code)]
    #[serde(rename = "m.login.token")]
    Token { token: String },
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    pub access_token: Option<AccessToken>,
    pub user_id: Option<UserId>,
    pub device_id: Box<DeviceId>,
}

pub fn request(base: Url, body: &Body) -> Result<Request, Error> {
    let url = base
        .join("_matrix/client/r0/login")
        .expect("Malformed URL in login");

    let data = serde_json::to_vec(body).unwrap();

    Client::new().post(url).body(data).build()
}
