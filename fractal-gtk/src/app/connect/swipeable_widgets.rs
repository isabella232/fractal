use gio::prelude::*;
use gtk::prelude::*;
use libhandy::prelude::*;

use crate::app::App;

impl App {
    // Set up HdyDeck and HdyLeaflet so that swipes trigger the
    // same behaviour as the back button.
    pub fn connect_swipeable_widgets(&self) {
        let deck: libhandy::Deck = self
            .ui
            .builder
            .get_object("main_deck")
            .expect("Can't find main_deck in UI file");
        let leaflet: libhandy::Leaflet = self
            .ui
            .builder
            .get_object("chat_page")
            .expect("Can't find chat_page in UI file");

        let app = gio::Application::get_default()
            .expect("Could not get default application")
            .downcast::<gtk::Application>()
            .unwrap();
        let global_back = app
            .lookup_action("back")
            .expect("Could not get back action");

        deck.connect_property_transition_running_notify(
            clone!(@weak app, @weak global_back => move |deck| {
                let child: Option<String> = deck.get_visible_child_name().map(|g| g.to_string());
                if !deck.get_transition_running() && child == Some("chat".to_string()) {
                    // Re-enable global back when returning to main view
                    let _ = global_back.set_property("enabled", &true);
                    app.activate_action("back", None);
                }
            }),
        );

        deck.connect_property_visible_child_notify(
            clone!(@weak app, @weak global_back => move |deck| {
                let child: Option<String> = deck.get_visible_child_name().map(|g| g.to_string());
                if !deck.get_transition_running() && child == Some("chat".to_string()) {
                    let _ = global_back.set_property("enabled", &true);
                    app.activate_action("back", None);
                }
            }),
        );

        leaflet.connect_property_child_transition_running_notify(
            clone!(@weak app => move |leaflet| {
                let child: Option<String> = leaflet.get_visible_child_name().map(|g| g.to_string());
                if !leaflet.get_child_transition_running() && child == Some("sidebar".to_string()) {
                    app.activate_action("back", None);
                }
            }),
        );

        leaflet.connect_property_visible_child_notify(clone!(@weak app => move |leaflet| {
            let child: Option<String> = deck.get_visible_child_name().map(|g| g.to_string());
            if !leaflet.get_child_transition_running() && child == Some("sidebar".to_string()) {
                app.activate_action("back", None);
            }
        }));
    }
}
