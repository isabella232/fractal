use super::UI;
use crate::widgets::ErrorDialog;
use gtk::prelude::*;

impl UI {
    pub fn inapp_notify(&self, msg: &str) {
        let inapp: gtk::Revealer = self
            .builder
            .get_object("inapp_revealer")
            .expect("Can't find inapp_revealer in ui file.");
        let label: gtk::Label = self
            .builder
            .get_object("inapp_label")
            .expect("Can't find inapp_label in ui file.");
        label.set_text(msg);
        inapp.set_reveal_child(true);
    }

    pub fn hide_inapp_notify(&self) {
        let inapp: gtk::Revealer = self
            .builder
            .get_object("inapp_revealer")
            .expect("Can't find inapp_revealer in ui file.");
        inapp.set_reveal_child(false);
    }

    pub fn show_error(&self, msg: String) {
        ErrorDialog::new(false, &msg);
    }

    pub fn show_error_with_info(&self, msg: String, info: Option<String>) {
        let dialog = ErrorDialog::new(false, &msg);
        if let Some(text) = info {
            dialog.set_property_secondary_text(Some(text.as_ref()));
        }
    }
}
