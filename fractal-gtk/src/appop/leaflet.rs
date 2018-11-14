use libhandy::*;

use appop::AppOp;
use appop::RoomPanel;

#[derive(Clone, Debug)]
pub enum LeafletState {
    Content,
    Sidebar
}

impl AppOp {
    pub fn set_leaflet_state(&mut self, state: LeafletState) {
        match state {
            LeafletState::Content =>  {
                self.header_leaflet.set_visible_child_name("room_header_bar");
                self.chat_state.set_visible_child_name("inapp");
            },
            LeafletState::Sidebar => {
                self.room_panel(RoomPanel::NoRoom);
                self.active_room = None;
                self.header_leaflet.set_visible_child_name("left-header");
                self.chat_state.set_visible_child_name("sidebar-box");
            }
        }
    }
}
