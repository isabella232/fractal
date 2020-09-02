use crate::backend::{user, HandleError};
use crate::util::i18n::i18n;
use gio::prelude::*;
use gio::SimpleAction;
use gio::SimpleActionGroup;
use glib::clone;
use std::sync::{Arc, Mutex};
use std::thread;

use crate::app::App;
use crate::appop::AppOp;

use crate::widgets::FileDialog::open;

use crate::actions::ButtonState;

// This creates all actions a user can perform in the account settings
pub fn new(window: &gtk::Window, op: Arc<Mutex<AppOp>>) -> gio::SimpleActionGroup {
    let actions = SimpleActionGroup::new();
    // TODO create two stats loading interaction and connect it to the avatar box
    let change_avatar =
        SimpleAction::new_stateful("change-avatar", None, &ButtonState::Sensitive.into());

    actions.add_action(&change_avatar);

    change_avatar.connect_activate(clone!(@weak window => move |a, _| {
        let login_data = unwrap_or_unit_return!(op.lock().unwrap().login_data.clone());
        let server_url = login_data.server_url;
        let access_token = login_data.access_token;
        let uid = login_data.uid;

        let filter = gtk::FileFilter::new();
        filter.add_mime_type("image/*");
        filter.set_name(Some(i18n("Images").as_str()));
        if let Some(path) = open(&window, i18n("Select a new avatar").as_str(), &[filter]) {
            a.change_state(&ButtonState::Insensitive.into());
            thread::spawn(move || {
                match user::set_user_avatar(server_url, access_token, uid, path) {
                    Ok(path) => {
                        APPOP!(show_new_avatar, (path));
                    }
                    Err(err) => {
                        err.handle_error();
                    }
                }
            });
        }
    }));

    actions
}
