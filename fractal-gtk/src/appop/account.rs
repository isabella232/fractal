use super::LoginData;
use crate::actions::global::activate_action;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::appop::AppState;
use crate::backend::user;
use crate::backend::HandleError;
use crate::cache::remove_from_cache;
use crate::util::i18n::i18n;
use gtk::prelude::*;
use log::info;
use matrix_sdk::api::r0::contact::get_contacts::ThirdPartyIdentifier;
use std::path::PathBuf;

impl AppOp {
    pub fn set_three_pid(&self, data: Option<Vec<ThirdPartyIdentifier>>) {
        self.ui.account_settings.set_three_pid(data, self);
    }

    pub fn get_three_pid(&self) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        RUNTIME.spawn(async move {
            match user::get_threepid(session_client).await {
                Ok(list) => {
                    let l = Some(list);
                    APPOP!(set_three_pid, (l));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }

    pub fn added_three_pid(&self) {
        self.get_three_pid();
    }

    pub fn valid_phone_token(&self, sid: Option<String>, secret: Option<String>) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        if let Some(sid) = sid {
            if let Some(secret) = secret {
                RUNTIME.spawn(async move {
                    let query = user::add_threepid(
                        login_data.session_client.homeserver().clone(),
                        login_data.access_token,
                        login_data.identity_url,
                        secret,
                        sid,
                    )
                    .await;

                    match query {
                        Ok(_) => {
                            APPOP!(added_three_pid);
                        }
                        Err(err) => {
                            err.handle_error();
                        }
                    }
                });
            }
        } else {
            self.show_error_dialog_in_settings(i18n("The validation code is not correct."));
        }
    }

    pub fn show_phone_dialog(&self, sid: String, secret: String) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        let entry = gtk::Entry::new();
        let msg = i18n("Enter the code received via SMS");
        let flags = gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT;
        let dialog = gtk::MessageDialog::new(
            Some(&self.ui.main_window),
            flags,
            gtk::MessageType::Error,
            gtk::ButtonsType::None,
            &msg,
        );
        if let Some(area) = dialog.get_message_area() {
            if let Ok(area) = area.downcast::<gtk::Box>() {
                area.add(&entry);
            }
        }
        dialog.add_button(&i18n("Cancel"), gtk::ResponseType::Cancel);
        let button = dialog.add_button(&i18n("Continue"), gtk::ResponseType::Ok);
        button.set_sensitive(false);
        let ok = button.clone();
        entry.connect_activate(move |_| {
            if ok.get_sensitive() {
                let _ = ok.emit("clicked", &[]);
            }
        });

        entry.connect_property_text_notify(move |w| {
            if !w.get_text().is_empty() {
                button.set_sensitive(true);
                return;
            }
            button.set_sensitive(false);
        });

        let value = entry;
        dialog.connect_response(move |w, r| {
            if let gtk::ResponseType::Ok = r {
                let token = value.get_text().to_string();

                let server_url = login_data.session_client.homeserver().clone();
                let secret = secret.clone();
                let sid = sid.clone();
                RUNTIME.spawn(async move {
                    match user::submit_phone_token(server_url, secret, sid, token).await {
                        Ok((sid, secret)) => {
                            let secret = Some(secret);
                            APPOP!(valid_phone_token, (sid, secret));
                        }
                        Err(err) => {
                            err.handle_error();
                        }
                    }
                });
            }
            w.close();
        });
        self.get_three_pid();
        dialog.show_all();
    }

    pub fn show_email_dialog(&self, sid: String, secret: String) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        let msg = i18n("In order to add this email address, go to your inbox and follow the link you received. Once you’ve done that, click Continue.");
        let flags = gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT;
        let dialog = gtk::MessageDialog::new(
            Some(&self.ui.main_window),
            flags,
            gtk::MessageType::Error,
            gtk::ButtonsType::None,
            &msg,
        );
        dialog.add_button(&i18n("Cancel"), gtk::ResponseType::Cancel);
        dialog.add_button(&i18n("Continue"), gtk::ResponseType::Ok);
        dialog.connect_response(move |w, r| {
            if let gtk::ResponseType::Ok = r {
                let login_data = login_data.clone();
                let secret = secret.clone();
                let sid = sid.clone();
                RUNTIME.spawn(async move {
                    let query = user::add_threepid(
                        login_data.session_client.homeserver().clone(),
                        login_data.access_token,
                        login_data.identity_url,
                        secret,
                        sid,
                    )
                    .await;

                    match query {
                        Ok(_) => {
                            APPOP!(added_three_pid);
                        }
                        Err(err) => {
                            err.handle_error();
                        }
                    }
                });
            }
            w.close();
        });
        self.get_three_pid();
        dialog.show_all();
    }

    pub fn show_error_dialog_in_settings(&self, error: String) {
        self.get_three_pid();
        self.ui
            .account_settings
            .show_error_dialog_in_settings(&self.ui, &error);
    }

    pub fn show_load_settings_error_dialog(&self, error: String) {
        self.ui
            .account_settings
            .show_load_settings_error_dialog(&self.ui, &error);
        activate_action(&self.app_runtime, "app", "back");
    }

    pub fn get_token_email(&mut self, sid: Option<String>, secret: Option<String>) {
        if let Some(sid) = sid {
            if let Some(secret) = secret {
                self.show_email_dialog(sid, secret);
            }
        }
    }

    pub fn get_token_phone(&mut self, sid: Option<String>, secret: Option<String>) {
        if let Some(sid) = sid {
            if let Some(secret) = secret {
                self.show_phone_dialog(sid, secret);
            }
        }
    }

    pub fn show_account_settings_dialog(&mut self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        self.get_three_pid();
        self.set_state(AppState::AccountSettings);

        self.ui.account_settings.show_dialog(
            login_data.session_client,
            self.user_info_cache.clone(),
            login_data.uid,
            login_data.username,
            &login_data.device_id,
        );
    }

    pub fn show_password_dialog(&self) {
        self.ui
            .account_settings
            .show_password_dialog(&self.ui.builder);
    }

    pub fn show_new_avatar(&mut self, path: PathBuf) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        info!("Request finished");
        self.set_login_data(LoginData {
            avatar: Some(path),
            ..login_data.clone()
        });
        remove_from_cache(self.user_info_cache.clone(), &login_data.uid);
        self.ui.account_settings.show_new_avatar(
            login_data.session_client,
            self.user_info_cache.clone(),
            login_data.uid,
            login_data.username,
        );
    }

    pub fn show_new_username(&mut self, name: Option<String>) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        self.set_login_data(LoginData {
            username: name.clone(),
            ..login_data
        });

        if let Some(name) = name.as_deref() {
            self.ui.account_settings.show_new_username(name);
        }
    }

    pub fn update_username_account_settings(&self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        let old_username = login_data.username.clone().unwrap_or_default();
        let username: String = self.ui.account_settings.name.get_text().into();

        if old_username != username {
            let spinner = gtk::Spinner::new();
            spinner.start();
            self.ui
                .account_settings
                .name_button
                .set_image(Some(&spinner));
            self.ui.account_settings.name_button.set_sensitive(false);
            self.ui.account_settings.name.set_editable(false);
            RUNTIME.spawn(async move {
                let query = user::set_username(
                    login_data.session_client,
                    &login_data.uid,
                    Some(username).filter(|u| !u.is_empty()),
                )
                .await;

                match query {
                    Ok(username) => {
                        APPOP!(show_new_username, (username));
                    }
                    Err(err) => {
                        err.handle_error();
                    }
                }
            });
        } else {
            self.ui.account_settings.name_button.hide();
        }
    }

    pub fn set_new_password(&mut self) {
        let (session_client, user_id) = unwrap_or_unit_return!(self
            .login_data
            .as_ref()
            .map(|ld| (ld.session_client.clone(), ld.uid.clone())));
        let old_password = self
            .ui
            .builder
            .get_object::<gtk::Entry>("password-dialog-old-entry")
            .expect("Can't find password-dialog-old-entry in ui file.");
        let new_password = self
            .ui
            .builder
            .get_object::<gtk::Entry>("password-dialog-entry")
            .expect("Can't find password-dialog-entry in ui file.");

        let old: String = old_password.get_text().into();
        let new: String = new_password.get_text().into();
        if !old.is_empty() && !new.is_empty() {
            self.ui.account_settings.password.set_sensitive(false);
            self.ui
                .account_settings
                .password_stack
                .set_visible_child_name("spinner");
            RUNTIME.spawn(async move {
                match user::change_password(session_client, &user_id, old, &new).await {
                    Ok(_) => {
                        APPOP!(password_changed);
                    }
                    Err(err) => {
                        err.handle_error();
                    }
                }
            });
        }
    }

    pub fn password_changed(&self) {
        self.ui.account_settings.password_changed();
    }

    pub fn show_password_error_dialog(&self, error: String) {
        self.ui
            .account_settings
            .show_password_error_dialog(&self.ui, &error);
    }

    pub fn close_password_dialog(&mut self) {
        self.ui
            .account_settings
            .close_password_dialog(&self.ui.builder);
    }

    pub fn account_destruction(&self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        let msg = i18n("Are you sure you want to delete your account?");
        let flags = gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT;
        let dialog = gtk::MessageDialog::new(
            Some(&self.ui.main_window),
            flags,
            gtk::MessageType::Warning,
            gtk::ButtonsType::None,
            &msg,
        );

        dialog.add_button("Confirm", gtk::ResponseType::Ok);
        dialog.add_button("Cancel", gtk::ResponseType::Cancel);

        // TODO: This is not used, remove?
        let _flag = self.ui.account_settings.delete_check.get_active();
        let password = self
            .ui
            .account_settings
            .delete_password_confirm
            .get_text()
            .to_string();
        dialog.connect_response(move |w, r| {
            if let gtk::ResponseType::Ok = r {
                let password = password.clone();
                let login_data = login_data.clone();
                RUNTIME.spawn(async move {
                    let query = user::account_destruction(
                        login_data.session_client.homeserver().clone(),
                        login_data.access_token.clone(),
                        login_data.uid.localpart().into(),
                        password,
                    )
                    .await;

                    match query {
                        Ok(_) => {
                            APPOP!(account_destruction_logoff);
                        }
                        Err(err) => {
                            err.handle_error();
                        }
                    }
                });
            }
            w.close();
        });
        dialog.show_all();
    }
    pub fn account_destruction_logoff(&self) {
        /* Do logout */
    }
}
