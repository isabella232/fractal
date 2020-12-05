use crate::actions::AppState;
use crate::appop::AppOp;

impl AppOp {
    pub fn create_room_settings(&mut self) -> Option<()> {
        let (session_client, user_id) = self
            .login_data
            .as_ref()
            .map(|ld| (ld.session_client.clone(), ld.uid.clone()))?;
        let room = self.rooms.get(self.active_room.as_ref()?).cloned()?;
        self.ui.create_room_settings(session_client, user_id, room);
        self.set_state(AppState::RoomSettings);
        None
    }

    pub fn show_new_room_avatar(&self) -> Option<()> {
        self.ui.show_new_room_avatar()
    }

    pub fn show_new_room_name(&self) -> Option<()> {
        self.ui.show_new_room_name()
    }

    pub fn show_new_room_topic(&self) -> Option<()> {
        self.ui.show_new_room_topic()
    }

    pub fn set_notifications_switch(&self, active: bool, sensitive: bool) -> Option<()> {
        self.ui.set_notifications_switch(active, sensitive)
    }
}
