use fractal_api::identifiers::{Error as IdentifierError, ServerName};
use fractal_api::url::ParseError as UrlError;
use fractal_api::Client as MatrixClient;
use fractal_api::Error as MatrixError;
use std::convert::{TryFrom, TryInto};

use crate::globals;

use crate::backend::MediaError;
use crate::util::cache_dir_path;

use crate::model::room::Room;
use fractal_api::api::r0::directory::get_public_rooms_filtered::Request as PublicRoomsFilteredRequest;
use fractal_api::api::r0::thirdparty::get_protocols::Request as GetProtocolsRequest;
use fractal_api::assign;
use fractal_api::directory::Filter as PublicRoomsFilter;
use fractal_api::directory::RoomNetwork;
use fractal_api::thirdparty::ProtocolInstance;

use super::{dw_media, ContentType, HandleError};
use crate::app::App;
use crate::util::i18n::i18n;
use crate::APPOP;

#[derive(Debug)]
pub struct DirectoryProtocolsError;

impl From<MatrixError> for DirectoryProtocolsError {
    fn from(_: MatrixError) -> Self {
        Self
    }
}

impl HandleError for DirectoryProtocolsError {
    fn handle_error(&self) {
        let error = i18n("Error searching for rooms");
        APPOP!(reset_directory_state);
        APPOP!(show_error, (error));
    }
}

pub async fn protocols(
    session_client: MatrixClient,
) -> Result<Vec<ProtocolInstance>, DirectoryProtocolsError> {
    Ok(session_client
        .send(GetProtocolsRequest::new())
        .await?
        .protocols
        .into_iter()
        .flat_map(|(_, protocol)| protocol.instances)
        .collect())
}

#[derive(Debug)]
pub enum DirectorySearchError {
    Matrix(MatrixError),
    MalformedServerName(IdentifierError),
    ParseUrl(UrlError),
    Download(MediaError),
}

impl From<MatrixError> for DirectorySearchError {
    fn from(err: MatrixError) -> Self {
        Self::Matrix(err)
    }
}

impl From<UrlError> for DirectorySearchError {
    fn from(err: UrlError) -> Self {
        Self::ParseUrl(err)
    }
}

impl From<MediaError> for DirectorySearchError {
    fn from(err: MediaError) -> Self {
        Self::Download(err)
    }
}

impl HandleError for DirectorySearchError {
    fn handle_error(&self) {
        let error = i18n("Error searching for rooms");
        APPOP!(reset_directory_state);
        APPOP!(show_error, (error));
    }
}

pub async fn room_search(
    session_client: MatrixClient,
    server: Option<&str>,
    search_term: Option<&str>,
    room_network: RoomNetwork<'_>,
    rooms_since: Option<&str>,
) -> Result<(Vec<Room>, Option<String>), DirectorySearchError> {
    let server = server
        .map(<&ServerName>::try_from)
        .transpose()
        .map_err(DirectorySearchError::MalformedServerName)?;

    let request = assign!(PublicRoomsFilteredRequest::new(), {
        server,
        limit: Some(globals::ROOM_DIRECTORY_LIMIT.into()),
        since: rooms_since,
        filter: assign!(PublicRoomsFilter::new(), {
            generic_search_term: search_term,
        }),
        room_network,
    });

    let response = session_client.public_rooms_filtered(request).await?;

    let since = response.next_batch;
    let rooms = response
        .chunk
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<Vec<Room>, UrlError>>()?;

    for room in &rooms {
        if let Some(avatar) = room.avatar.as_ref() {
            if let Ok(dest) = cache_dir_path(None, room.id.as_str()) {
                let _ = dw_media(
                    session_client.clone(),
                    avatar,
                    ContentType::Download,
                    Some(dest),
                )
                .await;
            }
        }
    }

    Ok((rooms, since))
}
