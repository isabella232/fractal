use fractal_api::api::error::ErrorKind as RumaErrorKind;
use fractal_api::identifiers::UserId;
use fractal_api::reqwest::Error as ReqwestError;
use fractal_api::url::{ParseError as UrlError, Url};
use fractal_api::{Client as MatrixClient, Error as MatrixError};
use std::collections::BTreeMap;
use std::io::Error as IoError;

use super::MediaError;
use crate::actions::global::activate_action;
use crate::appop::UserInfoCache;
use crate::backend::HTTP_CLIENT;
use crate::util::cache_dir_path;
use log::error;
use std::convert::TryInto;
use std::path::PathBuf;

use super::room::AttachedFileError;
use crate::model::member::Member;
use fractal_api::api::r0::account::change_password::Request as ChangePasswordRequest;
use fractal_api::api::r0::account::request_3pid_management_token_via_email::Request as EmailTokenRequest;
use fractal_api::api::r0::account::request_3pid_management_token_via_msisdn::Request as PhoneTokenRequest;
use fractal_api::api::r0::contact::get_contacts::Request as GetContactsRequest;
use fractal_api::api::r0::contact::get_contacts::ThirdPartyIdentifier;
use fractal_api::api::r0::profile::get_display_name::Request as GetDisplayNameRequest;
use fractal_api::api::r0::profile::get_profile::Request as GetProfileRequest;
use fractal_api::api::r0::profile::set_avatar_url::Request as SetAvatarUrlRequest;
use fractal_api::api::r0::profile::set_display_name::Request as SetDisplayNameRequest;
use fractal_api::api::r0::uiaa::AuthData;
use fractal_api::api::r0::user_directory::search_users::Request as UserDirectoryRequest;
use fractal_api::assign;
use fractal_api::identity::r0::association::msisdn::submit_token::request as submit_phone_token_req;
use fractal_api::identity::r0::association::msisdn::submit_token::Body as SubmitPhoneTokenBody;
use fractal_api::identity::r0::association::msisdn::submit_token::Response as SubmitPhoneTokenResponse;
use fractal_api::r0::account::deactivate::request as deactivate;
use fractal_api::r0::account::deactivate::Body as DeactivateBody;
use fractal_api::r0::account::deactivate::Parameters as DeactivateParameters;
use fractal_api::r0::account::AuthenticationData;
use fractal_api::r0::account::Identifier;
use fractal_api::r0::account::UserIdentifier;
use fractal_api::r0::contact::create::request as create_contact;
use fractal_api::r0::contact::create::Body as AddThreePIDBody;
use fractal_api::r0::contact::create::Parameters as AddThreePIDParameters;
use fractal_api::r0::contact::delete::request as delete_contact;
use fractal_api::r0::contact::delete::Body as DeleteThreePIDBody;
use fractal_api::r0::contact::delete::Parameters as DeleteThreePIDParameters;
use fractal_api::r0::AccessToken;
use fractal_api::r0::Medium;
use fractal_api::r0::ThreePIDCredentials;

use super::{dw_media, ContentType};

use super::{get_ruma_error_kind, remove_matrix_access_token_if_present, HandleError};
use crate::app::App;
use crate::util::i18n::i18n;
use crate::APPOP;
use serde_json::json;

pub type UserInfo = (String, PathBuf);

#[derive(Debug)]
pub struct NameError(MatrixError);

impl From<MatrixError> for NameError {
    fn from(err: MatrixError) -> Self {
        Self(err)
    }
}

impl HandleError for NameError {}

pub async fn get_username(
    session_client: MatrixClient,
    user_id: &UserId,
) -> Result<Option<String>, NameError> {
    let request = GetDisplayNameRequest::new(user_id);
    let response = session_client.send(request).await?;

    Ok(response.displayname)
}

#[derive(Debug)]
pub struct SetUserNameError(MatrixError);

impl From<MatrixError> for SetUserNameError {
    fn from(err: MatrixError) -> Self {
        Self(err)
    }
}

impl HandleError for SetUserNameError {}

pub async fn set_username(
    session_client: MatrixClient,
    user_id: &UserId,
    username: Option<String>,
) -> Result<Option<String>, SetUserNameError> {
    let request = SetDisplayNameRequest::new(user_id, username.as_ref().map(String::as_str));
    session_client.send(request).await?;

    Ok(username)
}

#[derive(Debug)]
pub struct GetThreePIDError;

impl From<MatrixError> for GetThreePIDError {
    fn from(_: MatrixError) -> Self {
        Self
    }
}

impl HandleError for GetThreePIDError {
    fn handle_error(&self) {
        let error = i18n("Sorry, account settings can’t be loaded.");
        APPOP!(show_load_settings_error_dialog, (error));
        let ctx = glib::MainContext::default();
        ctx.invoke(move || {
            activate_action("app", "back");
        })
    }
}

pub async fn get_threepid(
    session_client: MatrixClient,
) -> Result<Vec<ThirdPartyIdentifier>, GetThreePIDError> {
    let response = session_client.send(GetContactsRequest::new()).await?;

    Ok(response.threepids)
}

#[derive(Debug)]
pub struct GetTokenEmailError(MatrixError);

impl From<MatrixError> for GetTokenEmailError {
    fn from(err: MatrixError) -> Self {
        Self(err)
    }
}

impl HandleError for GetTokenEmailError {
    fn handle_error(&self) {
        let err = &self.0;
        let ruma_error_kind = get_ruma_error_kind(err);

        if ruma_error_kind == Some(&RumaErrorKind::ThreepidInUse) {
            let error = i18n("Email is already in use");
            APPOP!(show_error_dialog_in_settings, (error));
        } else if ruma_error_kind == Some(&RumaErrorKind::ThreepidDenied) {
            let error = i18n("Please enter a valid email address.");
            APPOP!(show_error_dialog_in_settings, (error));
        } else {
            let error = i18n("Couldn’t add the email address.");
            let err_str = format!("{:?}", err);
            error!(
                "{}",
                remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
            );
            APPOP!(show_error_dialog_in_settings, (error));
        }
    }
}

pub async fn get_email_token(
    session_client: MatrixClient,
    email: &str,
    client_secret: String,
) -> Result<(String, String), GetTokenEmailError> {
    let request = EmailTokenRequest::new(&client_secret, email, 1_u32.into());
    let response = session_client.send(request).await?;

    Ok((response.sid, client_secret))
}

#[derive(Debug)]
pub struct GetTokenPhoneError(MatrixError);

impl From<MatrixError> for GetTokenPhoneError {
    fn from(err: MatrixError) -> Self {
        Self(err)
    }
}

impl HandleError for GetTokenPhoneError {
    fn handle_error(&self) {
        let err = &self.0;
        let ruma_error_kind = get_ruma_error_kind(err);

        if ruma_error_kind == Some(&RumaErrorKind::ThreepidInUse) {
            let error = i18n("Phone number is already in use");
            APPOP!(show_error_dialog_in_settings, (error));
        } else if ruma_error_kind == Some(&RumaErrorKind::ThreepidDenied) {
            let error = i18n(
                "Please enter your phone number in the format: \n + your country code and your phone number.",
            );
            APPOP!(show_error_dialog_in_settings, (error));
        } else {
            let error = i18n("Couldn’t add the phone number.");
            let err_str = format!("{:?}", err);
            error!(
                "{}",
                remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
            );
            APPOP!(show_error_dialog_in_settings, (error));
        }
    }
}

pub async fn get_phone_token(
    session_client: MatrixClient,
    phone_number: &str,
    client_secret: String,
) -> Result<(String, String), GetTokenPhoneError> {
    let request = PhoneTokenRequest::new(&client_secret, "", phone_number, 1_u32.into());
    let response = session_client.send(request).await?;

    Ok((response.sid, client_secret))
}

#[derive(Debug)]
pub enum AddedToFavError {
    IdentityServerUrl(UrlError),
    Reqwest(ReqwestError),
}

impl From<ReqwestError> for AddedToFavError {
    fn from(err: ReqwestError) -> Self {
        Self::Reqwest(err)
    }
}

impl HandleError for AddedToFavError {}

pub fn add_threepid(
    base: Url,
    access_token: AccessToken,
    identity: Url,
    client_secret: String,
    sid: String,
) -> Result<(), AddedToFavError> {
    let params = AddThreePIDParameters { access_token };
    let body = AddThreePIDBody {
        three_pid_creds: ThreePIDCredentials {
            id_server: identity
                .try_into()
                .map_err(AddedToFavError::IdentityServerUrl)?,
            sid,
            client_secret,
        },
        bind: true,
    };

    let request = create_contact(base, &params, &body)?;
    HTTP_CLIENT.get_client().execute(request)?;

    Ok(())
}

#[derive(Debug)]
pub struct SubmitPhoneTokenError(ReqwestError);

impl From<ReqwestError> for SubmitPhoneTokenError {
    fn from(err: ReqwestError) -> Self {
        Self(err)
    }
}

impl HandleError for SubmitPhoneTokenError {}

pub fn submit_phone_token(
    base: Url,
    client_secret: String,
    sid: String,
    token: String,
) -> Result<(Option<String>, String), SubmitPhoneTokenError> {
    let body = SubmitPhoneTokenBody {
        sid: sid.clone(),
        client_secret: client_secret.clone(),
        token,
    };

    let request = submit_phone_token_req(base, &body)?;
    let response: SubmitPhoneTokenResponse = HTTP_CLIENT.get_client().execute(request)?.json()?;

    Ok((Some(sid).filter(|_| response.success), client_secret))
}

#[derive(Debug)]
pub struct DeleteThreePIDError(ReqwestError);

impl From<ReqwestError> for DeleteThreePIDError {
    fn from(err: ReqwestError) -> Self {
        Self(err)
    }
}

impl HandleError for DeleteThreePIDError {}

pub fn delete_three_pid(
    base: Url,
    access_token: AccessToken,
    medium: Medium,
    address: String,
) -> Result<(), DeleteThreePIDError> {
    let params = DeleteThreePIDParameters { access_token };
    let body = DeleteThreePIDBody { address, medium };

    let request = delete_contact(base, &params, &body)?;
    HTTP_CLIENT.get_client().execute(request)?;

    Ok(())
}

#[derive(Debug)]
pub struct ChangePasswordError(MatrixError);

impl From<MatrixError> for ChangePasswordError {
    fn from(err: MatrixError) -> Self {
        Self(err)
    }
}

impl HandleError for ChangePasswordError {
    fn handle_error(&self) {
        let error = i18n("Couldn’t change the password");
        let err_str = format!("{:?}", self);
        error!(
            "{}",
            remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
        );
        APPOP!(show_password_error_dialog, (error));
    }
}

pub async fn change_password(
    session_client: MatrixClient,
    user_id: &UserId,
    old_password: String,
    new_password: &str,
) -> Result<(), ChangePasswordError> {
    let auth_parameters = {
        let mut param = BTreeMap::new();
        let identifier = json!({
            "type": "m.id.user",
            "user": user_id.localpart(),
        });

        param.insert(String::from("identifier"), identifier);
        param.insert(String::from("password"), json!(old_password));

        param
    };

    let request = assign!(ChangePasswordRequest::new(new_password), {
        auth: Some(AuthData::DirectRequest {
            kind: "m.login.password",
            session: None,
            auth_parameters,
        }),
    });

    session_client.send(request).await?;

    Ok(())
}

#[derive(Debug)]
pub struct AccountDestructionError(ReqwestError);

impl From<ReqwestError> for AccountDestructionError {
    fn from(err: ReqwestError) -> Self {
        Self(err)
    }
}

impl HandleError for AccountDestructionError {
    fn handle_error(&self) {
        let error = i18n("Couldn’t delete the account");
        let err_str = format!("{:?}", self.0);
        error!(
            "{}",
            remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
        );
        APPOP!(show_error_dialog_in_settings, (error));
    }
}

pub fn account_destruction(
    base: Url,
    access_token: AccessToken,
    user: String,
    password: String,
) -> Result<(), AccountDestructionError> {
    let params = DeactivateParameters { access_token };
    let body = DeactivateBody {
        auth: Some(AuthenticationData::Password {
            identifier: Identifier::new(UserIdentifier::User { user }),
            password,
            session: None,
        }),
    };

    let request = deactivate(base, &params, &body)?;
    HTTP_CLIENT.get_client().execute(request)?;

    Ok(())
}

#[derive(Debug)]
pub enum SetUserAvatarError {
    Io(IoError),
    Matrix(MatrixError),
    ParseUrl(UrlError),
}

impl From<MatrixError> for SetUserAvatarError {
    fn from(err: MatrixError) -> Self {
        Self::Matrix(err)
    }
}

impl From<AttachedFileError> for SetUserAvatarError {
    fn from(err: AttachedFileError) -> Self {
        match err {
            AttachedFileError::Io(err) => Self::Io(err),
            AttachedFileError::Matrix(err) => Self::Matrix(err),
            AttachedFileError::ParseUrl(err) => Self::ParseUrl(err),
        }
    }
}

impl HandleError for SetUserAvatarError {}

pub async fn set_user_avatar(
    session_client: MatrixClient,
    user_id: &UserId,
    avatar: PathBuf,
) -> Result<PathBuf, SetUserAvatarError> {
    let avatar_url = super::room::upload_file(session_client.clone(), &avatar)
        .await?
        .content_uri;

    let request = SetAvatarUrlRequest::new(user_id, Some(&avatar_url));
    session_client.send(request).await?;

    Ok(avatar)
}

pub async fn get_user_info(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    uid: UserId,
) -> Result<UserInfo, GetUserAvatarError> {
    if let Some(info) = user_info_cache.lock().unwrap().get(&uid).cloned() {
        return Ok(info);
    }

    let info = get_user_avatar(session_client, &uid).await;

    if let Ok(ref i0) = info {
        user_info_cache.lock().unwrap().insert(uid, i0.clone());
    }

    info
}

#[derive(Debug)]
pub enum UserSearchError {
    Matrix(MatrixError),
    ParseUrl(UrlError),
}

impl From<MatrixError> for UserSearchError {
    fn from(err: MatrixError) -> Self {
        Self::Matrix(err)
    }
}

impl From<UrlError> for UserSearchError {
    fn from(err: UrlError) -> Self {
        Self::ParseUrl(err)
    }
}

impl HandleError for UserSearchError {}

pub async fn search(
    session_client: MatrixClient,
    search_term: &str,
) -> Result<Vec<Member>, UserSearchError> {
    let request = UserDirectoryRequest::new(search_term);
    let response = session_client.send(request).await?;

    response
        .results
        .into_iter()
        .map(TryInto::try_into)
        .collect::<Result<_, UrlError>>()
        .map_err(Into::into)
}

#[derive(Debug)]
pub enum GetUserAvatarError {
    Matrix(MatrixError),
    Download(MediaError),
    ParseUrl(UrlError),
}

impl From<MatrixError> for GetUserAvatarError {
    fn from(err: MatrixError) -> Self {
        Self::Matrix(err)
    }
}

impl From<MediaError> for GetUserAvatarError {
    fn from(err: MediaError) -> Self {
        Self::Download(err)
    }
}

impl From<UrlError> for GetUserAvatarError {
    fn from(err: UrlError) -> Self {
        Self::ParseUrl(err)
    }
}

impl HandleError for GetUserAvatarError {}

pub async fn get_user_avatar(
    session_client: MatrixClient,
    user_id: &UserId,
) -> Result<(String, PathBuf), GetUserAvatarError> {
    let request = GetProfileRequest::new(user_id);
    let response = session_client.send(request).await?;

    let img = match response
        .avatar_url
        .map(|url| Url::parse(&url))
        .transpose()?
        .map(|url| {
            (
                url,
                cache_dir_path(None, user_id.as_str()).map_err(MediaError::from),
            )
        }) {
        Some((url, Ok(dest))) => {
            dw_media(
                session_client,
                &url,
                ContentType::default_thumbnail(),
                Some(dest),
            )
            .await
        }
        Some((_, Err(err))) => Err(err),
        None => Ok(Default::default()),
    }?;

    let name = response
        .displayname
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| user_id.as_str().to_owned());

    Ok((name, img))
}
