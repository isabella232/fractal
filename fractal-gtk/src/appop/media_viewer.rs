use gtk::prelude::*;

use crate::actions;

use crate::appop::AppOp;
use crate::appop::AppState;

use crate::widgets;

use crate::model::message::Message;

impl AppOp {
    /* FIXME: take msg by reference and maybe create an action for this */
    pub fn create_media_viewer(&mut self, msg: Message) -> Option<()> {
        let login_data = self.login_data.clone()?;
        let stack = self
            .ui
            .builder
            .get_object::<gtk::Stack>("subview_stack")
            .expect("Can't find subview_stack in ui file.");

        let main_window = self
            .ui
            .builder
            .get_object::<gtk::Window>("main_window")
            .expect("Can't find main_window in ui file.");

        {
            let room_id = self.active_room.as_ref()?;
            let room = self.rooms.get(room_id)?;
            let mut panel = widgets::MediaViewer::new(main_window, room, &msg, login_data.uid);
            panel.display_media_viewer(login_data.session_client.clone(), msg);
            let (body, header) = panel.create(login_data.session_client.clone())?;
            self.ui.media_viewer = Some(panel);

            let actions = actions::Message::new(self.app_runtime.clone());
            header.insert_action_group("message", Some(&actions));
            body.insert_action_group("message", Some(&actions));

            /* remove old panel */
            if let Some(widget) = stack.get_child_by_name("media-viewer") {
                stack.remove(&widget);
            }

            stack.add_named(&body, "media-viewer");
        }

        self.set_state(AppState::MediaViewer);

        None
    }
}
