use crate::r0::filter::serialize_room_event_filter_as_str;
use crate::r0::filter::RoomEventFilter;
use crate::r0::u64_is_10;
use crate::r0::AccessToken;
use matrix_sdk::identifiers::{EventId, RoomId};
use reqwest::blocking::Client;
use reqwest::blocking::Request;
use reqwest::Error;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use url::Url;

#[derive(Clone, Debug, Serialize)]
pub struct Parameters<'a> {
    pub access_token: AccessToken,
    #[serde(skip_serializing_if = "u64_is_10")]
    pub limit: u64,
    #[serde(serialize_with = "serialize_room_event_filter_as_str")]
    #[serde(skip_serializing_if = "RoomEventFilter::is_default")]
    pub filter: RoomEventFilter<'a>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Response {
    pub start: Option<String>,
    pub end: Option<String>,
    #[serde(default)]
    pub events_before: Vec<JsonValue>,
    #[serde(default)]
    pub event: JsonValue,
    #[serde(default)]
    pub events_after: Vec<JsonValue>,
    #[serde(default)]
    pub state: Vec<JsonValue>,
}

pub fn request(
    base: Url,
    params: &Parameters,
    room_id: &RoomId,
    event_id: &EventId,
) -> Result<Request, Error> {
    let url = base
        .join(&format!(
            "_matrix/client/r0/rooms/{}/context/{}",
            room_id, event_id,
        ))
        .expect("Malformed URL in get_context");

    Client::new().get(url).query(params).build()
}
