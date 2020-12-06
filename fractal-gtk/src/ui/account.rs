use gtk::prelude::*;

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
}
