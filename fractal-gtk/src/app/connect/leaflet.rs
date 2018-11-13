use gtk::{self, prelude::*};
use libhandy::*;

use app::App;

impl App {
    pub fn connect_leaflet(&self) {
        let container = self.ui.builder
            .get_object::<gtk::Box>("message_column")
            .expect("Can't find message_column in ui file.");
        let chat_state_leaflet: libhandy::Leaflet = self.ui.builder
            .get_object("chat_state")
            .expect("Can't find chat_state in ui file.");

        let weak_container = container.downgrade();
        chat_state_leaflet.connect_property_fold_notify(move |leaflet| {
            weak_container.upgrade().map(|container| {
                match leaflet.get_fold() {
                    Fold::Folded => container.get_style_context().unwrap().add_class("folded-history"),
                    Fold::Unfolded => container.get_style_context().unwrap().remove_class("folded-history"),
                    _ => ()
                }
            });
        });
    }
}
