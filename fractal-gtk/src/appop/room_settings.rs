use gtk::prelude::*;

use crate::actions::AppState;
use crate::appop::AppOp;

use crate::widgets;

impl AppOp {
    pub fn create_room_settings(&mut self) -> Option<()> {
        let login_data = self.login_data.clone()?;
        let window = self
            .ui
            .builder
            .get_object::<gtk::Window>("main_window")
            .expect("Can't find main_window in ui file.");
        let stack = self
            .ui
            .builder
            .get_object::<gtk::Stack>("subview_stack")
            .expect("Can't find subview_stack in ui file.");

        {
            let room = self.rooms.get(&self.active_room.clone()?)?;
            let mut panel = widgets::RoomSettings::new(
                &window,
                login_data.uid,
                room.clone(),
                login_data.server_url,
                login_data.access_token,
            );
            let page = panel.create()?;

            /* remove old panel */
            if let Some(widget) = stack.get_child_by_name("room-settings") {
                stack.remove(&widget);
            }

            stack.add_named(&page, "room-settings");

            self.room_settings = Some(panel);
        }

        self.set_state(AppState::RoomSettings);

        None
    }

    pub fn show_new_room_avatar(&self) -> Option<()> {
        let panel = self.room_settings.clone()?;
        panel.show_new_room_avatar();
        None
    }

    pub fn show_new_room_name(&self) -> Option<()> {
        let panel = self.room_settings.clone()?;
        panel.show_new_room_name();
        None
    }

    pub fn show_new_room_topic(&self) -> Option<()> {
        let panel = self.room_settings.clone()?;
        panel.show_new_room_topic();
        None
    }

    pub fn set_notifications_switch(&self, active: bool, sensitive: bool) -> Option<()> {
        let panel = self.room_settings.clone()?;
        panel.set_notifications_switch(active, sensitive);
        None
    }
}
