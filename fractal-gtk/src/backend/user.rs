use fractal_api::identifiers::UserId;
use fractal_api::reqwest::Error as ReqwestError;
use fractal_api::url::{ParseError as UrlError, Url};
use fractal_api::{Client as MatrixClient, Error as MatrixError};
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
use fractal_api::api::r0::profile::get_display_name::Request as GetDisplayNameRequest;
use fractal_api::api::r0::profile::get_profile::Request as GetProfileRequest;
use fractal_api::api::r0::profile::set_avatar_url::Request as SetAvatarUrlRequest;
use fractal_api::api::r0::profile::set_display_name::Request as SetDisplayNameRequest;
use fractal_api::api::r0::user_directory::search_users::Request as UserDirectoryRequest;
use fractal_api::identity::r0::association::msisdn::submit_token::request as submit_phone_token_req;
use fractal_api::identity::r0::association::msisdn::submit_token::Body as SubmitPhoneTokenBody;
use fractal_api::identity::r0::association::msisdn::submit_token::Response as SubmitPhoneTokenResponse;
use fractal_api::r0::account::change_password::request as change_password_req;
use fractal_api::r0::account::change_password::Body as ChangePasswordBody;
use fractal_api::r0::account::change_password::Parameters as ChangePasswordParameters;
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
use fractal_api::r0::contact::get_identifiers::request as get_identifiers;
use fractal_api::r0::contact::get_identifiers::Parameters as ThirdPartyIDParameters;
use fractal_api::r0::contact::get_identifiers::Response as ThirdPartyIDResponse;
use fractal_api::r0::contact::get_identifiers::ThirdPartyIdentifier;
use fractal_api::r0::contact::request_verification_token_email::request as request_contact_verification_token_email;
use fractal_api::r0::contact::request_verification_token_email::Body as EmailTokenBody;
use fractal_api::r0::contact::request_verification_token_email::Parameters as EmailTokenParameters;
use fractal_api::r0::contact::request_verification_token_email::Response as EmailTokenResponse;
use fractal_api::r0::contact::request_verification_token_msisdn::request as request_contact_verification_token_msisdn;
use fractal_api::r0::contact::request_verification_token_msisdn::Body as PhoneTokenBody;
use fractal_api::r0::contact::request_verification_token_msisdn::Parameters as PhoneTokenParameters;
use fractal_api::r0::contact::request_verification_token_msisdn::Response as PhoneTokenResponse;
use fractal_api::r0::AccessToken;
use fractal_api::r0::Medium;
use fractal_api::r0::ThreePIDCredentials;

use super::{dw_media, ContentType};

use super::{remove_matrix_access_token_if_present, HandleError};
use crate::app::App;
use crate::util::i18n::i18n;
use crate::APPOP;

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

impl From<ReqwestError> for GetThreePIDError {
    fn from(_: ReqwestError) -> Self {
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

pub fn get_threepid(
    base: Url,
    access_token: AccessToken,
) -> Result<Vec<ThirdPartyIdentifier>, GetThreePIDError> {
    let params = ThirdPartyIDParameters { access_token };

    let request = get_identifiers(base, &params)?;
    let response: ThirdPartyIDResponse = HTTP_CLIENT.get_client().execute(request)?.json()?;

    Ok(response.threepids)
}

#[derive(Debug)]
pub enum GetTokenEmailError {
    IdentityServerUrl(UrlError),
    Reqwest(ReqwestError),
    TokenUsed,
    Denied,
}

impl From<ReqwestError> for GetTokenEmailError {
    fn from(err: ReqwestError) -> Self {
        Self::Reqwest(err)
    }
}

impl HandleError for GetTokenEmailError {
    fn handle_error(&self) {
        match self {
            Self::TokenUsed => {
                let error = i18n("Email is already in use");
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::Denied => {
                let error = i18n("Please enter a valid email address.");
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::Reqwest(err) => {
                let error = i18n("Couldn’t add the email address.");
                let err_str = format!("{:?}", err);
                error!(
                    "{}",
                    remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
                );
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::IdentityServerUrl(err) => {
                let error = i18n("The identity server is invalid.");
                error!("The identity server is invalid: {:?}", err);
                APPOP!(show_error_dialog_in_settings, (error));
            }
        }
    }
}

pub fn get_email_token(
    base: Url,
    access_token: AccessToken,
    identity: Url,
    email: String,
    client_secret: String,
) -> Result<(String, String), GetTokenEmailError> {
    use EmailTokenResponse::*;

    let params = EmailTokenParameters { access_token };
    let body = EmailTokenBody {
        id_server: identity
            .try_into()
            .map_err(GetTokenEmailError::IdentityServerUrl)?,
        client_secret: client_secret.clone(),
        email,
        send_attempt: 1,
        next_link: None,
    };

    let request = request_contact_verification_token_email(base, &params, &body)?;

    match HTTP_CLIENT
        .get_client()
        .execute(request)?
        .json::<EmailTokenResponse>()?
    {
        Passed(info) => Ok((info.sid, client_secret)),
        Failed(info) if info.errcode == "M_THREEPID_IN_USE" => Err(GetTokenEmailError::TokenUsed),
        Failed(_) => Err(GetTokenEmailError::Denied),
    }
}

#[derive(Debug)]
pub enum GetTokenPhoneError {
    IdentityServerUrl(UrlError),
    Reqwest(ReqwestError),
    TokenUsed,
    Denied,
}

impl From<ReqwestError> for GetTokenPhoneError {
    fn from(err: ReqwestError) -> Self {
        Self::Reqwest(err)
    }
}

impl HandleError for GetTokenPhoneError {
    fn handle_error(&self) {
        match self {
            Self::TokenUsed => {
                let error = i18n("Phone number is already in use");
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::Denied => {
                let error = i18n(
                    "Please enter your phone number in the format: \n + your country code and your phone number.",
                );
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::Reqwest(err) => {
                let error = i18n("Couldn’t add the phone number.");
                let err_str = format!("{:?}", err);
                error!(
                    "{}",
                    remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
                );
                APPOP!(show_error_dialog_in_settings, (error));
            }
            Self::IdentityServerUrl(err) => {
                let error = i18n("The identity server is invalid.");
                error!("The identity server is invalid: {:?}", err);
                APPOP!(show_error_dialog_in_settings, (error));
            }
        }
    }
}

pub fn get_phone_token(
    base: Url,
    access_token: AccessToken,
    identity: Url,
    phone_number: String,
    client_secret: String,
) -> Result<(String, String), GetTokenPhoneError> {
    use PhoneTokenResponse::*;

    let params = PhoneTokenParameters { access_token };
    let body = PhoneTokenBody {
        id_server: identity
            .try_into()
            .map_err(GetTokenPhoneError::IdentityServerUrl)?,
        client_secret: client_secret.clone(),
        phone_number,
        country: String::new(),
        send_attempt: 1,
        next_link: None,
    };

    let request = request_contact_verification_token_msisdn(base, &params, &body)?;

    match HTTP_CLIENT
        .get_client()
        .execute(request)?
        .json::<PhoneTokenResponse>()?
    {
        Passed(info) => Ok((info.sid, client_secret)),
        Failed(info) if info.errcode == "M_THREEPID_IN_USE" => Err(GetTokenPhoneError::TokenUsed),
        Failed(_) => Err(GetTokenPhoneError::Denied),
    }
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
pub struct ChangePasswordError(ReqwestError);

impl From<ReqwestError> for ChangePasswordError {
    fn from(err: ReqwestError) -> Self {
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

pub fn change_password(
    base: Url,
    access_token: AccessToken,
    user: String,
    old_password: String,
    new_password: String,
) -> Result<(), ChangePasswordError> {
    let params = ChangePasswordParameters { access_token };
    let body = ChangePasswordBody {
        new_password,
        auth: Some(AuthenticationData::Password {
            identifier: Identifier::new(UserIdentifier::User { user }),
            password: old_password,
            session: None,
        }),
    };

    let request = change_password_req(base, &params, &body)?;
    HTTP_CLIENT.get_client().execute(request)?;

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
