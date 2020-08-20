use crate::backend::room;
use fractal_api::identifiers::RoomId;
use fractal_api::Client as MatrixClient;
use gio::prelude::*;
use gio::SimpleAction;
use gio::SimpleActionGroup;
use glib::clone;
use std::convert::TryFrom;

use crate::app::{App, RUNTIME};
use crate::backend::HandleError;
use crate::util::i18n::i18n;

use crate::widgets::ErrorDialog;
use crate::widgets::FileDialog::open;

use crate::actions::ButtonState;

// This creates all actions a user can perform in the room settings
pub fn new(window: &gtk::Window, session_client: MatrixClient) -> gio::SimpleActionGroup {
    let actions = SimpleActionGroup::new();
    // TODO create two stats loading interaction and connect it to the avatar box
    let change_avatar = SimpleAction::new_stateful(
        "change-avatar",
        glib::VariantTy::new("s").ok(),
        &ButtonState::Sensitive.into(),
    );

    actions.add_action(&change_avatar);

    change_avatar.connect_activate(clone!(@weak window => move |a, data| {
        if let Some(room_id) = data
            .and_then(|x| x.get_str())
            .and_then(|rid| RoomId::try_from(rid).ok())
        {
            let filter = gtk::FileFilter::new();
            filter.set_name(Some(i18n("Images").as_str()));
            filter.add_mime_type("image/*");
            if let Some(file) = open(&window, i18n("Select a new avatar").as_str(), &[filter]) {
                a.change_state(&ButtonState::Insensitive.into());
                let session_client = session_client.clone();
                RUNTIME.spawn(async move {
                    match room::set_room_avatar(session_client, &room_id, &file).await {
                        Ok(_) => {
                            APPOP!(show_new_room_avatar);
                        }
                        Err(err) => {
                            err.handle_error();
                        }
                    }
                });
            } else {
                    ErrorDialog::new(false, &i18n("Couldn’t open file"));
            }
        }
    }));

    actions
}
