use super::LoginData;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::backend::{user, HandleError};
use std::path::PathBuf;

impl AppOp {
    pub fn get_username(&self) {
        let (session_client, user_id) = unwrap_or_unit_return!(self
            .login_data
            .as_ref()
            .map(|ld| (ld.session_client.clone(), ld.uid.clone())));

        let s_client = session_client.clone();
        let uid = user_id.clone();
        RUNTIME.spawn(async move {
            match user::get_username(s_client, &uid).await {
                Ok(username) => {
                    APPOP!(set_username, (username));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });

        RUNTIME.spawn(async move {
            match user::get_user_avatar(session_client, &user_id).await {
                Ok((_, path)) => {
                    APPOP!(set_avatar, (path));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }

    pub fn show_user_info(&self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        self.ui.show_user_info(
            login_data.session_client.clone(),
            self.user_info_cache.clone(),
            login_data.avatar.clone(),
            login_data.username.clone(),
            login_data.uid.clone(),
        );
    }

    pub fn set_login_data(&mut self, login_data: LoginData) {
        self.login_data = Some(login_data);
        self.show_user_info();
    }

    pub fn set_username(&mut self, username: Option<String>) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        self.set_login_data(LoginData {
            username,
            ..login_data
        });
    }

    pub fn set_avatar(&mut self, path: PathBuf) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        self.set_login_data(LoginData {
            avatar: Some(path),
            ..login_data
        });
    }
}
