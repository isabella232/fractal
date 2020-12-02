use log::error;

use crate::api::r0::AccessToken;
use crate::app::RUNTIME;
use crate::backend::register;
use matrix_sdk::identifiers::{DeviceId, ServerName, UserId};
use matrix_sdk::Session;
use url::Url;

use crate::appop::AppOp;

use crate::backend::HandleError;
use crate::cache;
use crate::client::get_matrix_client;

use crate::passwd::PasswordStorage;
use secret_service::SsError;

use crate::actions::AppState;

use super::LoginData;

impl AppOp {
    pub fn bk_login(
        &mut self,
        uid: UserId,
        access_token: AccessToken,
        device_id: Box<DeviceId>,
        server_url: Url,
        identity_url: Box<ServerName>,
    ) {
        match self.store_token(uid.clone(), access_token.clone()) {
            Err(SsError::Locked) => error!("Can’t store the token, keyring is locked."),
            Err(SsError::Dbus(_)) => error!("Can’t store the token, no Secret Service available."),
            _ => (),
        };

        let matrix_client =
            get_matrix_client(server_url).expect("Failed to login with the Matrix client");

        self.set_login_data(LoginData {
            session_client: matrix_client.clone(),
            uid: uid.clone(),
            access_token: access_token.clone(),
            device_id: device_id.clone(),
            username: None,
            avatar: None,
            identity_url,
        });

        let _ = RUNTIME
            .handle()
            .block_on(matrix_client.restore_login(Session {
                access_token: access_token.to_string(),
                user_id: uid,
                device_id,
            }));

        self.set_state(AppState::NoRoom);
        self.get_username();

        // initial sync, we're shoing some feedback to the user
        self.show_initial_sync();
        self.setup_sync();
        self.init_protocols();
    }

    pub fn bk_logout(&mut self) {
        self.set_rooms(vec![], true);
        if cache::get().destroy().is_err() {
            error!("Error removing cache file");
        }

        self.set_state(AppState::Login);
        self.login_data = None;
    }

    pub fn connect(
        &mut self,
        username: String,
        password: String,
        server: Url,
        identity: Box<ServerName>,
    ) {
        match self.store_pass(
            username.clone(),
            password.clone(),
            server.clone(),
            identity.clone(),
        ) {
            Err(SsError::Locked) => error!("Can’t store the password, keyring is locked."),
            Err(SsError::Dbus(_)) => {
                error!("Can’t store the password, no Secret Service available.")
            }
            _ => (),
        };

        RUNTIME.spawn(async move {
            match register::login(username, password, server.clone()).await {
                Ok((uid, tk, dev)) => {
                    APPOP!(bk_login, (uid, tk, dev, server, identity));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }

    // TODO: Remove function
    pub fn disconnect(&self) {}

    pub fn logout(&mut self) {
        let (homeserver, access_token) =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| (
                ld.session_client.homeserver().clone(),
                ld.access_token.clone()
            )));
        let _ = self.delete_secret("fractal");
        RUNTIME.spawn(async move {
            match register::logout(homeserver, access_token).await {
                Ok(_) => {
                    APPOP!(bk_logout);
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
        self.bk_logout();
        self.ui.room_back_history = vec![];
    }
}
