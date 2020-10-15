use crate::backend::{user, HandleError};
use crate::util::i18n::i18n;
use gio::prelude::*;
use gio::SimpleAction;
use gio::SimpleActionGroup;
use glib::clone;

use crate::app::{AppRuntime, RUNTIME};

use crate::widgets::FileDialog::open;

use crate::actions::ButtonState;

// This creates all actions a user can perform in the account settings
pub fn new(window: &gtk::Window, app_runtime: AppRuntime) -> gio::SimpleActionGroup {
    let actions = SimpleActionGroup::new();
    // TODO create two stats loading interaction and connect it to the avatar box
    let change_avatar =
        SimpleAction::new_stateful("change-avatar", None, &ButtonState::Sensitive.into());

    actions.add_action(&change_avatar);

    change_avatar.connect_activate(clone!(@weak window => move |a, _| {
        app_runtime.update_state_with(clone!(@weak a => move |state| {
            let (session_client, uid) = unwrap_or_unit_return!(
                state.login_data.as_ref().map(|ld| (ld.session_client.clone(), ld.uid.clone()))
            );

            let filter = gtk::FileFilter::new();
            filter.add_mime_type("image/*");
            filter.set_name(Some(i18n("Images").as_str()));
            if let Some(path) = open(&window, i18n("Select a new avatar").as_str(), &[filter]) {
                a.change_state(&ButtonState::Insensitive.into());
                RUNTIME.spawn(async move {
                    match user::set_user_avatar(session_client, &uid, path).await {
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
    }));

    actions
}
