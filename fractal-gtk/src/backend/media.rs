use super::MediaError;
use crate::globals;
use fractal_api::identifiers::{EventId, RoomId};
use fractal_api::url::Url;
use fractal_api::{Client as MatrixClient, Error as MatrixError};
use std::convert::TryInto;
use std::path::PathBuf;

use crate::model::message::Message;
use fractal_api::api::r0::filter::{RoomEventFilter, UrlFilter};
use fractal_api::api::r0::message::get_message_events::Request as GetMessagesEventsRequest;
use fractal_api::assign;

use super::{dw_media, get_prev_batch_from, ContentType};

pub type MediaResult = Result<PathBuf, MediaError>;
pub type MediaList = (Vec<Message>, String);

pub async fn get_thumb(session_client: MatrixClient, media: &Url) -> MediaResult {
    dw_media(
        session_client,
        media,
        ContentType::default_thumbnail(),
        None,
    )
    .await
}

pub async fn get_media(session_client: MatrixClient, media: &Url) -> MediaResult {
    dw_media(session_client, media, ContentType::Download, None).await
}

pub async fn get_media_list(
    session_client: MatrixClient,
    room_id: RoomId,
    first_media_id: EventId,
    prev_batch: Option<String>,
) -> Option<MediaList> {
    // FIXME: This should never be an empty token
    let from = match prev_batch {
        Some(prev_batch) => prev_batch,
        None => get_prev_batch_from(session_client.clone(), &room_id, &first_media_id)
            .await
            .ok()?,
    };

    get_room_media_list(session_client, &room_id, globals::PAGE_LIMIT, &from)
        .await
        .ok()
}

struct GetRoomMediaListError(MatrixError);

impl<T: Into<MatrixError>> From<T> for GetRoomMediaListError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

async fn get_room_media_list(
    session_client: MatrixClient,
    room_id: &RoomId,
    limit: u32,
    from: &str,
) -> Result<MediaList, GetRoomMediaListError> {
    let not_types = &["m.sticker".into()];

    let request = assign!(GetMessagesEventsRequest::backward(room_id, from), {
        to: None,
        limit: limit.into(),
        filter: Some(assign!(RoomEventFilter::empty(), {
            url_filter: Some(UrlFilter::EventsWithUrl),
            not_types,
        })),
    });

    let response = session_client.room_messages(request).await?;

    let prev_batch = response.end.unwrap_or_default();

    let media_list = response
        .chunk
        .into_iter()
        .rev()
        .filter_map(|ev| {
            ev.deserialize()
                .map(TryInto::try_into)
                .map(Result::ok)
                .transpose()
        })
        .collect::<Result<_, _>>()?;

    Ok((media_list, prev_batch))
}
