use crate::backend::user;
use gtk::prelude::*;
use log::info;
use std::path::PathBuf;

use crate::actions::global::activate_action;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::appop::AppState;
use crate::backend::HandleError;

use crate::util::i18n::i18n;
use crate::widgets;
use crate::widgets::AvatarExt;

use crate::cache::{download_to_cache, remove_from_cache};
use matrix_sdk::api::r0::contact::get_contacts::ThirdPartyIdentifier;
use matrix_sdk::thirdparty::Medium;

use super::LoginData;

impl AppOp {
    pub fn set_three_pid(&self, data: Option<Vec<ThirdPartyIdentifier>>) {
        self.update_address(data);
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
        let dialog = self.create_error_dialog(error);
        dialog.connect_response(move |w, _| w.close());
        self.get_three_pid();
        dialog.show_all();
    }

    pub fn show_load_settings_error_dialog(&self, error: String) {
        let dialog = self.create_error_dialog(error);
        dialog.connect_response(move |w, _| w.close());
        dialog.show_all();
        activate_action(&self.app_runtime, "app", "back");
    }

    pub fn create_error_dialog(&self, error: String) -> gtk::MessageDialog {
        let msg = error;
        let flags = gtk::DialogFlags::MODAL | gtk::DialogFlags::DESTROY_WITH_PARENT;
        let dialog = gtk::MessageDialog::new(
            Some(&self.ui.main_window),
            flags,
            gtk::MessageType::Error,
            gtk::ButtonsType::None,
            &msg,
        );

        dialog.add_button(&i18n("OK"), gtk::ResponseType::Ok);

        dialog
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
        // Reset view before displaying it
        self.close_account_settings_dialog();

        self.ui
            .account_settings
            .stack
            .set_visible_child_name("loading");
        self.get_three_pid();
        self.ui
            .account_settings
            .uid
            .set_text(&login_data.uid.to_string());
        self.ui
            .account_settings
            .device_id
            .set_text(login_data.device_id.as_str());
        self.ui
            .account_settings
            .homeserver
            .set_text(login_data.session_client.homeserver().as_str());
        self.ui
            .account_settings
            .name
            .set_text(&login_data.username.unwrap_or_default());
        self.ui.account_settings.name.grab_focus_without_selecting();
        self.ui.account_settings.name.set_position(-1);

        self.ui.account_settings.avatar_spinner.hide();
        self.ui.account_settings.avatar_button.set_sensitive(true);
        self.show_avatar();

        self.ui.account_settings.name_button.hide();
        self.ui.account_settings.name.set_editable(true);
        let image = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
        self.ui.account_settings.name_button.set_image(Some(&image));
        self.ui.account_settings.name_button.set_sensitive(true);

        // reset the password button
        self.ui
            .account_settings
            .password_stack
            .set_visible_child_name("label");
        self.ui.account_settings.password.set_sensitive(true);

        self.ui.account_settings.delete_check.set_active(false);
        self.ui.account_settings.delete_btn.set_sensitive(false);
        self.ui
            .account_settings
            .delete_password_confirm
            .set_text("");
        self.ui
            .account_settings
            .advanced_box
            .set_redraw_on_allocate(true);
        self.ui
            .account_settings
            .delete_box
            .set_redraw_on_allocate(true);

        self.set_state(AppState::AccountSettings);
    }

    pub fn update_address(&self, data: Option<Vec<ThirdPartyIdentifier>>) {
        let mut first_email = true;
        let mut first_phone = true;

        let grid = &self.ui.account_settings.grid;
        let mut child = grid.get_child_at(1, 1);
        let email = &self.ui.account_settings.email;
        let phone = &self.ui.account_settings.phone;
        let password = &self.ui.account_settings.password;

        let mut i = 1;
        while child.is_some() {
            if let Some(child) = child.as_ref() {
                if child != phone && child != email && child != password {
                    grid.remove_row(i);
                } else {
                    for w in email.get_children().iter() {
                        email.remove(w);
                    }
                    for w in phone.get_children().iter() {
                        phone.remove(w);
                    }
                    i += 1;
                }
            }
            child = grid.get_child_at(1, i);
        }

        /* Make sure we have at least one empty entry for email and phone */
        let mut empty_email = widgets::Address::new(widgets::AddressType::Email, &self);
        let mut empty_phone = widgets::Address::new(widgets::AddressType::Phone, &self);
        self.ui
            .account_settings
            .email
            .pack_start(&empty_email.create(None), true, true, 0);
        phone.pack_start(&empty_phone.create(None), true, true, 0);
        if let Some(data) = data {
            for item in data {
                match item.medium {
                    Medium::Email => {
                        if first_email {
                            empty_email.update(Some(item.address));
                            let entry = widgets::Address::new(widgets::AddressType::Email, &self)
                                .create(None);
                            grid.insert_next_to(email, gtk::PositionType::Bottom);
                            grid.attach_next_to(
                                &entry,
                                Some(email),
                                gtk::PositionType::Bottom,
                                1,
                                1,
                            );
                            first_email = false;
                        } else {
                            let entry = widgets::Address::new(widgets::AddressType::Email, &self)
                                .create(Some(item.address));
                            grid.insert_next_to(email, gtk::PositionType::Bottom);
                            grid.attach_next_to(
                                &entry,
                                Some(email),
                                gtk::PositionType::Bottom,
                                1,
                                1,
                            );
                        }
                    }
                    Medium::MSISDN => {
                        if first_phone {
                            empty_phone.update(Some(item.address));
                            let entry = widgets::Address::new(widgets::AddressType::Phone, &self)
                                .create(None);
                            grid.insert_next_to(phone, gtk::PositionType::Bottom);
                            grid.attach_next_to(
                                &entry,
                                Some(phone),
                                gtk::PositionType::Bottom,
                                1,
                                1,
                            );
                            first_phone = false;
                        } else {
                            let s = String::from("+") + &item.address;
                            let entry = widgets::Address::new(widgets::AddressType::Phone, &self)
                                .create(Some(s));
                            grid.insert_next_to(phone, gtk::PositionType::Bottom);
                            grid.attach_next_to(
                                &entry,
                                Some(phone),
                                gtk::PositionType::Bottom,
                                1,
                                1,
                            );
                        }
                    }
                    medium => log::warn!("Medium type not managed: {:?}", medium),
                }
            }
        }
        self.ui
            .account_settings
            .stack
            .set_visible_child_name("info");
    }

    pub fn show_password_dialog(&self) {
        let dialog = self
            .ui
            .builder
            .get_object::<gtk::Dialog>("password_dialog")
            .expect("Can't find password_dialog in ui file.");
        let confirm_password = self
            .ui
            .builder
            .get_object::<gtk::Button>("password-dialog-apply")
            .expect("Can't find password-dialog-apply in ui file.");
        confirm_password.set_sensitive(false);
        dialog.present();
    }

    pub fn show_new_avatar(&mut self, path: PathBuf) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());

        info!("Request finished");
        self.set_login_data(LoginData {
            avatar: Some(path),
            ..login_data
        });
        self.ui.account_settings.avatar_spinner.hide();
        self.ui.account_settings.avatar_button.set_sensitive(true);
        if let Some(login_data) = &self.login_data {
            remove_from_cache(self.user_info_cache.clone(), &login_data.uid);
        }
        self.show_avatar();
    }

    pub fn show_avatar(&self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        /* remove all old avatar */
        for w in self.ui.account_settings.avatar.get_children().iter() {
            if w != &self.ui.account_settings.avatar_spinner {
                self.ui.account_settings.avatar.remove(w);
            }
        }

        let w = widgets::Avatar::avatar_new(Some(100));
        self.ui.account_settings.avatar.add(&w);

        let data = w.circle(
            login_data.uid.to_string(),
            login_data.username,
            100,
            None,
            None,
        );
        download_to_cache(
            login_data.session_client,
            self.user_info_cache.clone(),
            login_data.uid,
            data,
        );

        /* FIXME: hack to make the avatar drawing area clickable*/
        let current = self.ui.account_settings.stack.get_visible_child_name();
        self.ui
            .account_settings
            .stack
            .set_visible_child_name("loading");
        if let Some(current) = current {
            self.ui
                .account_settings
                .stack
                .set_visible_child_name(&current);
        }
    }

    pub fn show_new_username(&mut self, name: Option<String>) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        if let Some(name) = name.clone() {
            self.ui.account_settings.name_button.hide();
            let image = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
            self.ui.account_settings.name_button.set_image(Some(&image));
            self.ui.account_settings.name_button.set_sensitive(true);
            self.ui.account_settings.name.set_editable(true);
            self.ui.account_settings.name.set_text(&name);
        }
        self.set_login_data(LoginData {
            username: name,
            ..login_data
        });
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

    pub fn close_account_settings_dialog(&self) {
        self.ui.account_settings.advanced_box.queue_draw();
        self.ui.account_settings.delete_box.queue_draw();
        self.ui.account_settings.root.queue_draw();
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
        self.ui.account_settings.password.set_sensitive(true);
        self.ui
            .account_settings
            .password_stack
            .set_visible_child_name("label");
    }

    pub fn show_password_error_dialog(&self, error: String) {
        self.show_error_dialog_in_settings(error);
        self.ui.account_settings.password.set_sensitive(true);
        self.ui
            .account_settings
            .password_stack
            .set_visible_child_name("label");
    }

    pub fn close_password_dialog(&mut self) {
        let dialog = self
            .ui
            .builder
            .get_object::<gtk::Dialog>("password_dialog")
            .expect("Can't find password_dialog in ui file.");
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
        let verify_password = self
            .ui
            .builder
            .get_object::<gtk::Entry>("password-dialog-verify-entry")
            .expect("Can't find password-dialog-verify-entry in ui file.");
        /* Clear all user input */
        old_password.set_text("");
        new_password.set_text("");
        verify_password.set_text("");
        dialog.hide();
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
