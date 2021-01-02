use super::UI;
use crate::actions;
use crate::app::AppRuntime;
use crate::model::message::Message;
use crate::model::room::Room;
use crate::widgets;
use gtk::prelude::*;
use matrix_sdk::{identifiers::UserId, Client as MatrixClient};

impl UI {
    // FIXME: take msg by reference and maybe create an action for this
    pub fn create_media_viewer(
        &mut self,
        app_runtime: AppRuntime,
        session_client: MatrixClient,
        room: &Room,
        user_id: UserId,
        msg: Message,
    ) -> Option<()> {
        let mut panel =
            widgets::MediaViewer::new(self.main_window.clone().upcast(), room, &msg, user_id);
        panel.display_media_viewer(session_client.clone(), msg);
        let (body, header) = panel.create(session_client)?;
        self.media_viewer = Some(panel);

        let actions = actions::Message::new(app_runtime);
        header.insert_action_group("message", Some(&actions));
        body.insert_action_group("message", Some(&actions));

        // remove old panel
        if let Some(widget) = self.subview_stack.get_child_by_name("media-viewer") {
            self.subview_stack.remove(&widget);
        }

        self.subview_stack.add_named(&body, "media-viewer");

        Some(())
    }
}
