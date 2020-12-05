use super::RoomSearchPagination;
use crate::actions::AppState;
use crate::appop::AppOp;
use gtk::prelude::*;

impl AppOp {
    pub fn set_state(&mut self, state: AppState) {
        self.state = state;

        match self.state {
            AppState::Login => self.ui.set_stack_state("login"),
            AppState::NoRoom => {
                self.ui.set_stack_state("main_view");
                self.ui.set_chat_state(None);
                self.active_room = None;
                self.clear_tmp_msgs();
            }
            AppState::Room => {
                let msg = if let Some(active_room_id) = self.active_room.as_ref() {
                    self.unsent_messages
                        .get(active_room_id)
                        .map(|(msg_text, msg_number)| (msg_text.as_str(), msg_number.clone()))
                        .unwrap_or_default()
                } else {
                    Default::default()
                };

                self.ui.set_stack_state("main_view");
                self.ui.set_chat_state(Some(msg));
            }
            AppState::Directory => self.ui.set_deck_state(Some("directory"), state),
            AppState::Loading => self.ui.set_stack_state("loading"),
            AppState::AccountSettings => self.ui.set_deck_state(Some("account-settings"), state),
            AppState::RoomSettings => self.ui.set_deck_state(Some("room-settings"), state),
            AppState::MediaViewer => self.ui.set_deck_state(Some("media-viewer"), state),
        };

        //set focus for room directory
        if let AppState::Directory = self.state {
            self.ui
                .builder
                .get_object::<gtk::Widget>("directory_search_entry")
                .expect("Can't find widget to set focus in ui file.")
                .grab_focus();
            self.directory_pagination = RoomSearchPagination::Initial;
            self.search_rooms();
        }
    }
}
