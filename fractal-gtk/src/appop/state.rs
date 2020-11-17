use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use super::RoomSearchPagination;
use crate::actions::AppState;
use crate::appop::AppOp;

impl AppOp {
    pub fn set_state(&mut self, state: AppState) {
        self.state = state;

        match self.state {
            AppState::Login => self.set_stack_state("login"),
            AppState::NoRoom | AppState::Room => {
                self.set_stack_state("main_view");
                self.set_chat_state(state);
            }
            AppState::Directory => self.set_deck_state(Some("directory"), state),
            AppState::Loading => self.set_stack_state("loading"),
            AppState::AccountSettings => self.set_deck_state(Some("account-settings"), state),
            AppState::RoomSettings => self.set_deck_state(Some("room-settings"), state),
            AppState::MediaViewer => self.set_deck_state(Some("media-viewer"), state),
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

    fn set_deck_state(&self, view: Option<&str>, state: AppState) {
        let deck = self
            .ui
            .builder
            .get_object::<libhandy::Deck>("main_deck")
            .expect("Could not find main_deck in ui file");
        let stack = self
            .ui
            .builder
            .get_object::<gtk::Stack>("subview_stack")
            .expect("Could not find subview_stack in ui file");
        let app = gio::Application::get_default().unwrap();

        let global_back = app.lookup_action("back");

        let direction = match state {
            AppState::Room | AppState::NoRoom => libhandy::NavigationDirection::Back,
            _ => libhandy::NavigationDirection::Forward,
        };

        if let Some(v) = view {
            stack.set_visible_child_name(v);
        }

        if deck.get_adjacent_child(direction).is_some() {
            deck.navigate(direction);
            if direction == libhandy::NavigationDirection::Forward {
                // Disable global back while in a subview
                global_back.map(|a| a.set_property("enabled", &false));
            }
        }
    }

    fn set_stack_state(&self, state: &str) {
        self.ui
            .builder
            .get_object::<gtk::Stack>("main_content_stack")
            .expect("Can't find main_content_stack in ui file.")
            .set_visible_child_name(state);
    }

    fn set_chat_state(&mut self, state: AppState) {
        let deck = self
            .ui
            .builder
            .get_object::<libhandy::Deck>("main_deck")
            .expect("Could not find main_deck in ui file");
        let stack = self
            .ui
            .builder
            .get_object::<gtk::Stack>("room_view_stack")
            .expect("Can't find room_view_stack in ui file.");
        let headerbar = self
            .ui
            .builder
            .get_object::<libhandy::HeaderBar>("room_header_bar")
            .expect("Can't find room_header_bar in ui file.");

        match state {
            AppState::NoRoom => {
                self.set_state_no_room(&headerbar);
                self.ui
                    .leaflet
                    .navigate(libhandy::NavigationDirection::Back);
                stack.set_visible_child_name("noroom");
            }
            AppState::Room => {
                self.set_state_room(&headerbar);
                self.ui
                    .leaflet
                    .navigate(libhandy::NavigationDirection::Forward);
                stack.set_visible_child_name("room_view");
            }
            _ => (),
        }

        if deck
            .get_adjacent_child(libhandy::NavigationDirection::Back)
            .is_some()
        {
            deck.navigate(libhandy::NavigationDirection::Back);
        }
    }

    fn set_state_room(&self, headerbar: &libhandy::HeaderBar) {
        for ch in headerbar.get_children().iter() {
            ch.show();
        }

        self.ui.sventry.view.grab_focus();

        let msg = self
            .active_room
            .as_ref()
            .and_then(|active_room_id| self.unsent_messages.get(active_room_id))
            .cloned()
            .unwrap_or_default();
        if let Some(buffer) = self.ui.sventry.view.get_buffer() {
            buffer.set_text(&msg.0);

            let iter = buffer.get_iter_at_offset(msg.1);
            buffer.place_cursor(&iter);
        }
    }

    // WORKAROUND this is needed because NoRoom isn't a real app state
    fn set_state_no_room(&mut self, headerbar: &libhandy::HeaderBar) {
        for ch in headerbar.get_children().iter() {
            ch.hide();

            // Select new active room in the sidebar
            self.ui.roomlist.unselect();
        }
        self.active_room = None;
        self.clear_tmp_msgs();
    }
}
