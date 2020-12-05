use super::UI;
use crate::appop::UserInfoCache;
use crate::cache::download_to_cache;
use crate::widgets;
use crate::widgets::AvatarExt;
use gtk::prelude::*;
use matrix_sdk::identifiers::UserId;
use matrix_sdk::Client as MatrixClient;
use std::path::PathBuf;

impl UI {
    pub fn show_user_info(
        &self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        avatar_path: Option<PathBuf>,
        username: Option<String>,
        user_id: UserId,
    ) {
        let stack = self
            .builder
            .get_object::<gtk::Stack>("user_info")
            .expect("Can't find user_info_avatar in ui file.");

        /* Show user infos inside the popover but wait for all data to arrive */
        if let (Some(username), Some(_)) = (username.clone(), avatar_path.as_ref()) {
            let avatar = self
                .builder
                .get_object::<gtk::Container>("user_info_avatar")
                .expect("Can't find user_info_avatar in ui file.");

            let name = self
                .builder
                .get_object::<gtk::Label>("user_info_username")
                .expect("Can't find user_info_avatar in ui file.");

            let uid = self
                .builder
                .get_object::<gtk::Label>("user_info_uid")
                .expect("Can't find user_info_avatar in ui file.");

            uid.set_text(user_id.as_ref());
            name.set_text(&username);

            /* remove all old avatar from the popover */
            for w in avatar.get_children().iter() {
                avatar.remove(w);
            }

            let w = widgets::Avatar::avatar_new(Some(40));
            let data = w.circle(user_id.to_string(), Some(username), 40, None, None);
            download_to_cache(
                session_client.clone(),
                user_info_cache.clone(),
                user_id.clone(),
                data,
            );

            avatar.add(&w);
            stack.set_visible_child_name("info");
        } else {
            stack.set_visible_child_name("spinner");
        }

        let eb = gtk::EventBox::new();
        match avatar_path {
            Some(_) => {
                let w = widgets::Avatar::avatar_new(Some(24));
                let data = w.circle(user_id.to_string(), username, 24, None, None);
                download_to_cache(session_client, user_info_cache, user_id, data);

                eb.add(&w);
            }
            None => {
                let w = gtk::Spinner::new();
                w.show();
                w.start();
                eb.add(&w);
            }
        };

        eb.connect_button_press_event(move |_, _| Inhibit(false));
    }
}
