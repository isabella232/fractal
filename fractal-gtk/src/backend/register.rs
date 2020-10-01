use matrix_sdk::identifiers::{DeviceId, UserId};
use matrix_sdk::reqwest::Error as ReqwestError;
use url::{ParseError as UrlError, Url};

use crate::actions::AppState;
use crate::api::r0::account::login::request as login_req;
use crate::api::r0::account::login::Auth;
use crate::api::r0::account::login::Body as LoginBody;
use crate::api::r0::account::login::Response as LoginResponse;
use crate::api::r0::account::logout::request as logout_req;
use crate::api::r0::account::logout::Parameters as LogoutParameters;
use crate::api::r0::account::Identifier;
use crate::api::r0::account::UserIdentifier;
use crate::api::r0::server::domain_info::request as domain_info;
use crate::api::r0::server::domain_info::Response as DomainInfoResponse;
use crate::api::r0::AccessToken;
use crate::api::r0::Medium;
use crate::backend::HTTP_CLIENT;
use crate::globals;

use super::HandleError;
use crate::app::App;
use crate::util::i18n::i18n;
use crate::APPOP;

#[derive(Debug)]
pub struct LoginError;

impl From<ReqwestError> for LoginError {
    fn from(_: ReqwestError) -> Self {
        Self
    }
}

impl From<serde_json::Error> for LoginError {
    fn from(_: serde_json::Error) -> Self {
        Self
    }
}

impl HandleError for LoginError {
    fn handle_error(&self) {
        let error = i18n("Can’t login, try again");
        let st = AppState::Login;
        APPOP!(show_error, (error));
        APPOP!(logout);
        APPOP!(set_state, (st));
    }
}

pub async fn login(
    user: String,
    password: String,
    server: Url,
) -> Result<(UserId, AccessToken, Box<DeviceId>), LoginError> {
    let body = if globals::EMAIL_RE.is_match(&user) {
        LoginBody {
            auth: Auth::Password { password },
            identifier: Identifier::new(UserIdentifier::ThirdParty {
                medium: Medium::Email,
                address: user,
            }),
            initial_device_display_name: Some(globals::DEVICE_NAME.into()),
            device_id: None,
        }
    } else {
        LoginBody {
            auth: Auth::Password { password },
            identifier: Identifier::new(UserIdentifier::User { user }),
            initial_device_display_name: Some(globals::DEVICE_NAME.into()),
            device_id: None,
        }
    };

    let request = login_req(server, &body)?;
    let response_raw = HTTP_CLIENT
        .get_client()
        .execute(request)
        .await?
        .bytes()
        .await?;

    let response: LoginResponse = serde_json::from_slice(&response_raw)?;

    if let (Some(tk), Some(uid)) = (response.access_token, response.user_id) {
        Ok((uid, tk, response.device_id))
    } else {
        Err(LoginError)
    }
}

#[derive(Debug)]
pub struct LogoutError(ReqwestError);

impl From<ReqwestError> for LogoutError {
    fn from(err: ReqwestError) -> Self {
        Self(err)
    }
}

impl HandleError for LogoutError {}

pub async fn logout(server: Url, access_token: AccessToken) -> Result<(), LogoutError> {
    let params = LogoutParameters { access_token };

    let request = logout_req(server, &params)?;
    HTTP_CLIENT.get_client().execute(request).await?;

    Ok(())
}

#[derive(Debug)]
pub enum GetWellKnownError {
    Reqwest(ReqwestError),
    Json(serde_json::Error),
    ParseUrl(UrlError),
}

impl From<ReqwestError> for GetWellKnownError {
    fn from(err: ReqwestError) -> Self {
        Self::Reqwest(err)
    }
}

impl From<serde_json::Error> for GetWellKnownError {
    fn from(err: serde_json::Error) -> Self {
        Self::Json(err)
    }
}

impl From<UrlError> for GetWellKnownError {
    fn from(err: UrlError) -> Self {
        Self::ParseUrl(err)
    }
}

pub async fn get_well_known(domain: Url) -> Result<DomainInfoResponse, GetWellKnownError> {
    let request = domain_info(domain)?;

    let response_raw = HTTP_CLIENT
        .get_client()
        .execute(request)
        .await?
        .bytes()
        .await?;

    serde_json::from_slice(&response_raw).map_err(Into::into)
}
