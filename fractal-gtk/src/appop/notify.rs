use crate::app::RUNTIME;
use crate::backend::user;
use gio::ApplicationExt;
use gio::FileExt;
use gio::Notification;
use gtk::prelude::*;
use log::info;
use matrix_sdk::identifiers::{EventId, RoomId};
use std::path::Path;

use crate::util::i18n::i18n;

use crate::appop::AppOp;

use crate::widgets::ErrorDialog;

impl AppOp {
    pub fn inapp_notify(&self, msg: &str) {
        let inapp: gtk::Revealer = self
            .ui
            .builder
            .get_object("inapp_revealer")
            .expect("Can't find inapp_revealer in ui file.");
        let label: gtk::Label = self
            .ui
            .builder
            .get_object("inapp_label")
            .expect("Can't find inapp_label in ui file.");
        label.set_text(msg);
        inapp.set_reveal_child(true);
    }

    pub fn hide_inapp_notify(&self) {
        let inapp: gtk::Revealer = self
            .ui
            .builder
            .get_object("inapp_revealer")
            .expect("Can't find inapp_revealer in ui file.");
        inapp.set_reveal_child(false);
    }

    pub fn notify(&self, app: gtk::Application, room_id: RoomId, id: EventId) -> Option<()> {
        let session_client = self.login_data.as_ref()?.session_client.clone();
        let msg = self.get_message_by_id(&room_id, &id)?;
        let r = self.rooms.get(&room_id)?;
        let short_body = match &msg.mtype[..] {
            "m.audio" => i18n("An audio file has been added to the conversation."),
            "m.image" => i18n("An image has been added to the conversation."),
            "m.video" => i18n("A video has been added to the conversation."),
            "m.file" => i18n("A file has been added to the conversation."),
            _ => dirty_truncate(&msg.body, 80).to_string(),
        };

        let title = if r.direct {
            i18n(" (direct message)")
        } else if let Some(name) = r.name.clone() {
            format!(" ({})", name)
        } else {
            String::new()
        };

        let response = RUNTIME.spawn(user::get_user_info(
            session_client,
            self.user_info_cache.clone(),
            msg.sender,
        ));

        glib::MainContext::default().spawn_local(async move {
            if let Ok(Ok((name, avatar_path))) = response.await {
                let title = format!("{}{}", name, title);
                let n = create_notification(room_id.as_str(), &title, &short_body, &avatar_path);
                app.send_notification(Some(id.as_str()), &n);
            }
        });

        None
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

fn dirty_truncate(s: &str, num_chars: usize) -> &str {
    let l = s.len();

    if l <= num_chars {
        s
    } else if let Some((idx, _ch)) = s.char_indices().find(|(idx, _ch)| *idx >= num_chars) {
        s.get(0..idx).unwrap()
    } else {
        s
    }
}

fn create_notification(room_id: &str, title: &str, body: &str, avatar: &Path) -> Notification {
    let notification = Notification::new(title);
    notification.set_body(Some(body));
    notification.set_priority(gio::NotificationPriority::High);
    info!("Creating notification with avatar: {}", avatar.display());
    let file = gio::File::new_for_path(avatar);
    let _ = file.load_bytes(gio::NONE_CANCELLABLE).map(|(b, _)| {
        let avatar = gio::BytesIcon::new(&b);
        notification.set_icon(&avatar);
    });
    let data = glib::Variant::from(room_id);
    notification.set_default_action_and_target_value("app.open-room", Some(&data));
    notification
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dirty_truncate_works() {
        assert_eq!(dirty_truncate("hello", 80), "hello");

        assert_eq!(
            dirty_truncate(
                "0123456789012345678901234567890123456789012345678901234567890123456789012345678áéíóú",
                80
            ),
            "0123456789012345678901234567890123456789012345678901234567890123456789012345678á"
        );

        // len 82, max index 79 for the ideograph
        assert_eq!(
            dirty_truncate(
                "0123456789012345678901234567890123456789012345678901234567890123456789012345678㈨",
                80
            ),
            "0123456789012345678901234567890123456789012345678901234567890123456789012345678㈨"
        );
    }
}
