use crate::r0::AccessToken;
use reqwest::blocking::Client;
use reqwest::blocking::Request;
use reqwest::Error;
use ruma_identifiers::{RoomId, RoomIdOrAliasId};
use serde::{Deserialize, Serialize};
use url::Host;
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters {
    pub access_token: AccessToken,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub server_name: Vec<Host>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    pub room_id: RoomId,
}

// TODO: Implement Body

pub fn request(
    base: Url,
    room_id_or_alias: &RoomIdOrAliasId,
    params: &Parameters,
) -> Result<Request, Error> {
    // econde # in the room id to allow alias
    let encoded_id = room_id_or_alias.to_string().replace("#", "%23");
    let url = base
        .join(&format!("_matrix/client/r0/join/{}", encoded_id))
        .expect("Malformed URL in join_room_by_id_or_alias");

    Client::new().post(url).query(params).build()
}
