use super::UI;
use crate::appop::AppOp;
use crate::appop::UserInfoCache;
use crate::cache::{download_to_cache, remove_from_cache};
use crate::widgets;
use crate::widgets::AvatarExt;
use gtk::prelude::*;
use matrix_sdk::api::r0::contact::get_contacts::ThirdPartyIdentifier;
use matrix_sdk::identifiers::{DeviceId, UserId};
use matrix_sdk::thirdparty::Medium;
use matrix_sdk::Client as MatrixClient;

pub struct AccountSettings {
    pub root: gtk::Box,
    pub advanced_box: gtk::Box,
    pub avatar: gtk::Overlay,
    pub avatar_button: gtk::Button,
    pub avatar_spinner: gtk::Spinner,
    pub delete_box: gtk::Box,
    pub delete_btn: gtk::Button,
    pub delete_check: gtk::CheckButton,
    pub delete_password_confirm: gtk::Entry,
    pub device_id: gtk::Label,
    pub email: gtk::Box,
    pub grid: gtk::Grid,
    pub homeserver: gtk::Label,
    pub name: gtk::Entry,
    pub name_button: gtk::Button,
    pub password: gtk::Button,
    pub password_stack: gtk::Stack,
    pub phone: gtk::Box,
    pub stack: gtk::Stack,
    pub uid: gtk::Label,
}

impl AccountSettings {
    pub fn new(builder: &gtk::Builder) -> Self {
        Self {
            root: builder
                .get_object("account_settings_box")
                .expect("Can't find account_settings_box in ui file."),
            advanced_box: builder
                .get_object("account_settings_advanced_box")
                .expect("Can't find account_settings_advanced_box in ui file."),
            avatar: builder
                .get_object("account_settings_avatar")
                .expect("Can't find account_settings_advanced_box in ui file."),
            avatar_button: builder
                .get_object("account_settings_avatar_button")
                .expect("Can't find account_settings_advanced_box in ui file."),
            avatar_spinner: builder
                .get_object("account_settings_avatar_spinner")
                .expect("Can't find account_settings_advanced_box in ui file."),
            delete_box: builder
                .get_object("account_settings_delete_box")
                .expect("Can't find account_settings_delete_box in ui file."),
            delete_btn: builder
                .get_object("account_settings_delete_btn")
                .expect("Can't find account_settings_delete_btn in ui file."),
            delete_check: builder
                .get_object("account_settings_delete_check")
                .expect("Can't find account_settings_delete_check in ui file."),
            delete_password_confirm: builder
                .get_object("account_settings_delete_password_confirm")
                .expect("Can't find account_settings_delete_password_confirm in ui file."),
            device_id: builder
                .get_object("account_settings_device_id")
                .expect("Can't find account_settings_device_id in ui file."),
            email: builder
                .get_object("account_settings_email")
                .expect("Can't find account_settings_email in ui file."),
            grid: builder
                .get_object("account_settings_grid")
                .expect("Can't find account_settings_grid in ui file."),
            homeserver: builder
                .get_object("account_settings_homeserver")
                .expect("Can't find account_settings_homeserver in ui file."),
            name: builder
                .get_object("account_settings_name")
                .expect("Can't find account_settings_name in ui file."),
            name_button: builder
                .get_object("account_settings_name_button")
                .expect("Can't find account_settings_name_button in ui file."),
            password: builder
                .get_object("account_settings_password")
                .expect("Can't find account_settings_password in ui file."),
            password_stack: builder
                .get_object("account_settings_password_stack")
                .expect("Can't find account_settings_password_stack in ui file."),
            phone: builder
                .get_object("account_settings_phone")
                .expect("Can't find account_settings_phone in ui file."),
            stack: builder
                .get_object("account_settings_stack")
                .expect("Can't find account_settings_stack in ui file."),
            uid: builder
                .get_object("account_settings_uid")
                .expect("Can't find account_settings_uid in ui file."),
        }
    }

    pub fn set_three_pid(&self, data: Option<Vec<ThirdPartyIdentifier>>, op: &AppOp) {
        let mut first_email = true;
        let mut first_phone = true;

        let mut maybe_child = self.grid.get_child_at(1, 1);

        let mut i = 1;
        while let Some(child) = maybe_child.as_ref() {
            if child != &self.phone && child != &self.email && child != &self.password {
                self.grid.remove_row(i);
            } else {
                for w in self.email.get_children().iter() {
                    self.email.remove(w);
                }
                for w in self.phone.get_children().iter() {
                    self.phone.remove(w);
                }
                i += 1;
            }
            maybe_child = self.grid.get_child_at(1, i);
        }

        /* Make sure we have at least one empty entry for email and phone */
        let mut empty_email = widgets::Address::new(widgets::AddressType::Email, op);
        let mut empty_phone = widgets::Address::new(widgets::AddressType::Phone, op);
        self.email
            .pack_start(&empty_email.create(None), true, true, 0);
        self.phone
            .pack_start(&empty_phone.create(None), true, true, 0);
        for item in data.unwrap_or_default() {
            match item.medium {
                Medium::Email => {
                    let item_address = if first_email {
                        empty_email.update(Some(item.address));
                        first_email = false;

                        None
                    } else {
                        Some(item.address)
                    };

                    let entry = widgets::Address::new(widgets::AddressType::Email, &op)
                        .create(item_address);
                    self.grid
                        .insert_next_to(&self.email, gtk::PositionType::Bottom);
                    self.grid.attach_next_to(
                        &entry,
                        Some(&self.email),
                        gtk::PositionType::Bottom,
                        1,
                        1,
                    );
                }
                Medium::MSISDN => {
                    let s = if first_phone {
                        empty_phone.update(Some(item.address));
                        first_phone = false;

                        None
                    } else {
                        Some(String::from("+") + &item.address)
                    };

                    let entry = widgets::Address::new(widgets::AddressType::Phone, op).create(s);
                    self.grid
                        .insert_next_to(&self.phone, gtk::PositionType::Bottom);
                    self.grid.attach_next_to(
                        &entry,
                        Some(&self.phone),
                        gtk::PositionType::Bottom,
                        1,
                        1,
                    );
                }
                medium => log::warn!("Medium type not managed: {:?}", medium),
            }
        }
        self.stack.set_visible_child_name("info");
    }

    pub fn show_error_dialog_in_settings(&self, ui: &UI, error_msg: &str) {
        let dialog = ui.create_error_dialog(error_msg);
        dialog.connect_response(move |w, _| w.close());
        dialog.show_all();
    }

    pub fn show_load_settings_error_dialog(&self, ui: &UI, error_msg: &str) {
        let dialog = ui.create_error_dialog(error_msg);
        dialog.connect_response(move |w, _| w.close());
        dialog.show_all();
    }

    pub fn show_dialog(
        &mut self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        user_id: UserId,
        username: Option<String>,
        device_id: &DeviceId,
    ) {
        // Reset view before displaying it
        self.close_dialog();

        self.stack.set_visible_child_name("loading");
        self.uid.set_text(user_id.as_str());
        self.device_id.set_text(device_id.as_str());
        self.homeserver
            .set_text(session_client.homeserver().as_str());
        self.name.set_text(username.as_deref().unwrap_or_default());
        self.name.grab_focus_without_selecting();
        self.name.set_position(-1);

        self.avatar_spinner.hide();
        self.avatar_button.set_sensitive(true);

        self.name_button.hide();
        self.name.set_editable(true);
        let image = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
        self.name_button.set_image(Some(&image));
        self.name_button.set_sensitive(true);

        // reset the password button
        self.password_stack.set_visible_child_name("label");
        self.password.set_sensitive(true);

        self.delete_check.set_active(false);
        self.delete_btn.set_sensitive(false);
        self.delete_password_confirm.set_text("");
        self.advanced_box.set_redraw_on_allocate(true);
        self.delete_box.set_redraw_on_allocate(true);

        self.show_avatar(session_client, user_info_cache, user_id, username);
    }

    pub fn show_password_dialog(&self, builder: &gtk::Builder) {
        let dialog = builder
            .get_object::<gtk::Dialog>("password_dialog")
            .expect("Can't find password_dialog in ui file.");
        let confirm_password = builder
            .get_object::<gtk::Button>("password-dialog-apply")
            .expect("Can't find password-dialog-apply in ui file.");
        confirm_password.set_sensitive(false);
        dialog.present();
    }

    fn show_avatar(
        &self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        user_id: UserId,
        username: Option<String>,
    ) {
        /* remove all old avatar */
        for w in self.avatar.get_children().iter() {
            if w != &self.avatar_spinner {
                self.avatar.remove(w);
            }
        }

        let w = widgets::Avatar::avatar_new(Some(100));
        self.avatar.add(&w);

        let data = w.circle(user_id.to_string(), username, 100, None, None);
        download_to_cache(session_client, user_info_cache, user_id, data);

        /* FIXME: hack to make the avatar drawing area clickable*/
        let current = self.stack.get_visible_child_name();
        self.stack.set_visible_child_name("loading");
        if let Some(current) = current {
            self.stack.set_visible_child_name(&current);
        }
    }

    pub fn show_new_avatar(
        &mut self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        user_id: UserId,
        username: Option<String>,
    ) {
        self.avatar_spinner.hide();
        self.avatar_button.set_sensitive(true);
        remove_from_cache(user_info_cache.clone(), &user_id);
        self.show_avatar(session_client, user_info_cache, user_id, username);
    }

    pub fn show_new_username(&mut self, name: &str) {
        self.name_button.hide();
        let image = gtk::Image::from_icon_name(Some("emblem-ok-symbolic"), gtk::IconSize::Menu);
        self.name_button.set_image(Some(&image));
        self.name_button.set_sensitive(true);
        self.name.set_editable(true);
        self.name.set_text(name);
    }

    fn close_dialog(&self) {
        self.advanced_box.queue_draw();
        self.delete_box.queue_draw();
        self.root.queue_draw();
    }

    pub fn password_changed(&self) {
        self.password.set_sensitive(true);
        self.password_stack.set_visible_child_name("label");
    }

    pub fn show_password_error_dialog(&self, ui: &UI, error_msg: &str) {
        self.show_error_dialog_in_settings(ui, error_msg);
        self.password.set_sensitive(true);
        self.password_stack.set_visible_child_name("label");
    }

    pub fn close_password_dialog(&mut self, builder: &gtk::Builder) {
        let dialog = builder
            .get_object::<gtk::Dialog>("password_dialog")
            .expect("Can't find password_dialog in ui file.");
        let old_password = builder
            .get_object::<gtk::Entry>("password-dialog-old-entry")
            .expect("Can't find password-dialog-old-entry in ui file.");
        let new_password = builder
            .get_object::<gtk::Entry>("password-dialog-entry")
            .expect("Can't find password-dialog-entry in ui file.");
        let verify_password = builder
            .get_object::<gtk::Entry>("password-dialog-verify-entry")
            .expect("Can't find password-dialog-verify-entry in ui file.");
        /* Clear all user input */
        old_password.set_text("");
        new_password.set_text("");
        verify_password.set_text("");
        dialog.hide();
    }
}
