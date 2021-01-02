use super::UI;
use crate::actions::AppState;
use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

impl UI {
    pub fn set_deck_state(&self, view: Option<&str>, state: AppState) {
        let deck = self
            .builder
            .get_object::<libhandy::Deck>("main_deck")
            .expect("Could not find main_deck in ui file");
        let app = gio::Application::get_default().unwrap();

        let global_back = app.lookup_action("back");

        let direction = match state {
            AppState::Room | AppState::NoRoom => libhandy::NavigationDirection::Back,
            _ => libhandy::NavigationDirection::Forward,
        };

        if let Some(v) = view {
            self.subview_stack.set_visible_child_name(v);
        }

        if deck.get_adjacent_child(direction).is_some() {
            deck.navigate(direction);
            if direction == libhandy::NavigationDirection::Forward {
                // Disable global back while in a subview
                global_back.map(|a| a.set_property("enabled", &false));
            }
        }
    }

    pub fn set_stack_state(&self, state: &str) {
        self.builder
            .get_object::<gtk::Stack>("main_content_stack")
            .expect("Can't find main_content_stack in ui file.")
            .set_visible_child_name(state);
    }

    pub fn set_chat_state(&mut self, msg: Option<(&str, i32)>) {
        let deck = self
            .builder
            .get_object::<libhandy::Deck>("main_deck")
            .expect("Could not find main_deck in ui file");
        let stack = self
            .builder
            .get_object::<gtk::Stack>("room_view_stack")
            .expect("Can't find room_view_stack in ui file.");
        let headerbar = self
            .builder
            .get_object::<libhandy::HeaderBar>("room_header_bar")
            .expect("Can't find room_header_bar in ui file.");

        match msg {
            None => {
                self.set_state_no_room(&headerbar);
                self.leaflet.navigate(libhandy::NavigationDirection::Back);
                stack.set_visible_child_name("noroom");
            }
            Some((msg_text, msg_number)) => {
                self.set_state_room(&headerbar, msg_text, msg_number);
                self.leaflet
                    .navigate(libhandy::NavigationDirection::Forward);
                stack.set_visible_child_name("room_view");
            }
        }

        if deck
            .get_adjacent_child(libhandy::NavigationDirection::Back)
            .is_some()
        {
            deck.navigate(libhandy::NavigationDirection::Back);
        }
    }

    fn set_state_room(&self, headerbar: &libhandy::HeaderBar, msg_text: &str, msg_number: i32) {
        for ch in headerbar.get_children().iter() {
            ch.show();
        }

        self.sventry.view.grab_focus();

        if let Some(buffer) = self.sventry.view.get_buffer() {
            buffer.set_text(msg_text);

            let iter = buffer.get_iter_at_offset(msg_number);
            buffer.place_cursor(&iter);
        }
    }

    // WORKAROUND this is needed because NoRoom isn't a real app state
    fn set_state_no_room(&mut self, headerbar: &libhandy::HeaderBar) {
        for ch in headerbar.get_children().iter() {
            ch.hide();

            // Select new active room in the sidebar
            self.roomlist.unselect();
        }
    }
}
