use fractal_api::identifiers::{EventId, RoomId, ServerName};
use fractal_api::reqwest::Error as ReqwestError;
use fractal_api::url::Url;
use fractal_api::{Client as MatrixClient, Error as MatrixError};
use lazy_static::lazy_static;
use log::error;
use regex::Regex;
use std::convert::TryFrom;
use std::fmt::Debug;
use std::io::Error as IoError;
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;

use crate::client::ClientBlocking;
use crate::util::cache_dir_path;
use fractal_api::api::r0::media::get_content::Request as GetContentRequest;
use fractal_api::api::r0::media::get_content_thumbnail::Method;
use fractal_api::api::r0::media::get_content_thumbnail::Request as GetContentThumbnailRequest;
use fractal_api::assign;
use fractal_api::r0::context::get_context::request as get_context;
use fractal_api::r0::context::get_context::Parameters as GetContextParameters;
use fractal_api::r0::context::get_context::Response as GetContextResponse;
use fractal_api::r0::AccessToken;

pub mod directory;
pub mod media;
pub mod register;
pub mod room;
pub mod sync;
pub mod user;

lazy_static! {
    pub static ref HTTP_CLIENT: ClientBlocking = ClientBlocking::new();
}

#[derive(Clone, Debug)]
pub struct ThreadPool {
    thread_count: Arc<(Mutex<u8>, Condvar)>,
    limit: u8,
}

impl ThreadPool {
    pub fn new(limit: u8) -> Self {
        ThreadPool {
            thread_count: Arc::new((Mutex::new(0), Condvar::new())),
            limit,
        }
    }

    pub fn run<F>(&self, func: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let thread_count = self.thread_count.clone();
        let limit = self.limit;
        thread::spawn(move || {
            // waiting, less than {limit} threads at the same time
            let &(ref num, ref cvar) = &*thread_count;
            {
                let mut start = num.lock().unwrap();
                while *start >= limit {
                    start = cvar.wait(start).unwrap()
                }
                *start += 1;
            }

            func();

            // freeing the cvar for new threads
            {
                let mut counter = num.lock().unwrap();
                *counter -= 1;
            }
            cvar.notify_one();
        });
    }
}

pub enum ContentType {
    Download,
    Thumbnail(u32, u32),
}

impl ContentType {
    pub fn default_thumbnail() -> Self {
        ContentType::Thumbnail(128, 128)
    }

    pub fn is_thumbnail(&self) -> bool {
        match self {
            ContentType::Download => false,
            ContentType::Thumbnail(_, _) => true,
        }
    }
}

pub fn get_prev_batch_from(
    base: Url,
    access_token: AccessToken,
    room_id: &RoomId,
    event_id: &EventId,
) -> Result<String, ReqwestError> {
    let params = GetContextParameters {
        access_token,
        limit: 0,
        filter: Default::default(),
    };

    let request = get_context(base, &params, room_id, event_id)?;
    let response: GetContextResponse = HTTP_CLIENT.get_client().execute(request)?.json()?;
    let prev_batch = response.start.unwrap_or_default();

    Ok(prev_batch)
}

#[derive(Debug)]
pub enum MediaError {
    MalformedMxcUrl,
    Io(IoError),
    Matrix(MatrixError),
}

impl From<MatrixError> for MediaError {
    fn from(err: MatrixError) -> Self {
        Self::Matrix(err)
    }
}

impl From<IoError> for MediaError {
    fn from(err: IoError) -> Self {
        Self::Io(err)
    }
}

impl HandleError for MediaError {}

pub async fn dw_media(
    session_client: MatrixClient,
    mxc: &Url,
    media_type: ContentType,
    dest: Option<PathBuf>,
) -> Result<PathBuf, MediaError> {
    if mxc.scheme() != "mxc" {
        return Err(MediaError::MalformedMxcUrl);
    }

    let server_name = mxc
        .host()
        .as_ref()
        .map(ToString::to_string)
        .and_then(|host| {
            <&ServerName>::try_from(host.as_str())
                .map(ToOwned::to_owned)
                .ok()
        })
        .ok_or(MediaError::MalformedMxcUrl)?;

    let media_id = mxc
        .path_segments()
        .and_then(|mut ps| ps.next())
        .filter(|s| !s.is_empty())
        .ok_or(MediaError::MalformedMxcUrl)?;

    let default_fname = || {
        let dir = if media_type.is_thumbnail() {
            "thumbs"
        } else {
            "medias"
        };
        cache_dir_path(Some(dir), &media_id)
    };
    let fname = dest.clone().map_or_else(default_fname, Ok)?;

    // If the file is already cached and recent enough, don't download it
    let is_fname_recent = fname
        .metadata()
        .ok()
        .and_then(|md| md.modified().ok())
        .and_then(|modf| modf.elapsed().ok())
        .map_or(false, |dur| dur.as_secs() < 60);

    if fname.is_file() && (dest.is_none() || is_fname_recent) {
        return Ok(fname);
    }

    let media = if let ContentType::Thumbnail(width, height) = media_type {
        let request = assign!(GetContentThumbnailRequest::new(
                &media_id,
                &server_name,
                width.into(),
                height.into(),
            ), {
            method: Some(Method::Crop),
        });

        session_client.send(request).await?.file
    } else {
        let request = GetContentRequest::new(&media_id, &server_name);
        session_client.send(request).await?.file
    };

    tokio::fs::write(&fname, media).await?;

    Ok(fname)
}

pub trait HandleError: Debug {
    #[track_caller]
    fn handle_error(&self) {
        let err_str = format!("{:?}", self);
        error!(
            "Query error: {}",
            remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
        );
    }
}

/// This function removes the value of the `access_token` query from a URL used for accessing the Matrix API.
/// The primary use case is the removing of sensitive information for logging.
/// Specifically, the URL is expected to be contained within quotes and the token is replaced with `<redacted>`.
/// Returns `Some` on removal, otherwise `None`.
pub fn remove_matrix_access_token_if_present(message: &str) -> Option<String> {
    lazy_static! {
    static ref RE: Regex =
        Regex::new(r#""((http)|(https))://([^"]+)/_matrix/([^"]+)\?access_token=(?P<token>[^&"]+)([^"]*)""#,)
        .expect("Malformed regular expression.");
    }
    // If the supplied string doesn't contain a match for the regex, we return `None`.
    let cap = RE.captures(message)?;
    let captured_token = cap
        .name("token")
        .expect("'token' capture group not present.")
        .as_str();
    Some(message.replace(captured_token, "<redacted>"))
}
