use crate::appop::AppOp;
use crate::appop::AppState;
use crate::model::message::Message;

impl AppOp {
    // FIXME: take msg by reference and maybe create an action for this
    pub fn create_media_viewer(&mut self, msg: Message) -> Option<()> {
        let login_data = self.login_data.as_ref()?;
        let active_room = self.active_room.as_ref()?;
        let room = self.rooms.get(active_room)?;

        self.ui.create_media_viewer(
            self.app_runtime.clone(),
            login_data.session_client.clone(),
            room,
            login_data.uid.clone(),
            msg,
        )?;

        self.set_state(AppState::MediaViewer);
        None
    }
}
