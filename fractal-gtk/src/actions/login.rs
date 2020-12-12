use libhandy::prelude::*;
use log::warn;

use gio::prelude::*;
use gio::SimpleAction;
use gio::SimpleActionGroup;
use glib::clone;
use gtk::prelude::*;

use crate::globals;

#[derive(Debug, Clone, PartialEq)]
pub enum LoginState {
    Greeter,
    ServerChooser,
    Credentials,
}

impl From<String> for LoginState {
    fn from(v: String) -> LoginState {
        match v.as_str() {
            "greeter" => LoginState::Greeter,
            "server-chooser" => LoginState::ServerChooser,
            "credentials" => LoginState::Credentials,
            _ => panic!("Invalid back state type"),
        }
    }
}

impl ToString for LoginState {
    fn to_string(&self) -> String {
        let str = match self {
            LoginState::Greeter => "greeter",
            LoginState::ServerChooser => "server-chooser",
            LoginState::Credentials => "credentials",
        };

        String::from(str)
    }
}

pub fn new(
    deck: &libhandy::Deck,
    server_entry: &gtk::Entry,
    err_label: &gtk::Label,
) -> SimpleActionGroup {
    let actions = SimpleActionGroup::new();

    let create_account = SimpleAction::new("create-account", None);
    let server_chooser = SimpleAction::new("server_chooser", None);
    let credentials = SimpleAction::new("credentials", None);
    let back = SimpleAction::new("back", None);
    let login = SimpleAction::new("login", None);

    actions.add_action(&create_account);
    actions.add_action(&server_chooser);
    actions.add_action(&credentials);
    actions.add_action(&back);
    actions.add_action(&login);

    create_account.connect_activate(clone!(@weak deck => move |_, _| {
        let toplevel = deck
            .get_toplevel()
            .expect("Could not grab toplevel widget")
            .downcast::<gtk::Window>()
            .expect("Could not cast toplevel to GtkWindow");
        let uri = globals::ELEMENT_REGISTER_URL;
        if let Err(e) = gtk::show_uri_on_window(Some(&toplevel), uri, gtk::get_current_event_time())
        {
            warn!("Could not show {}: {}", uri, e)
        }
    }));

    server_chooser.connect_activate(clone!(@weak deck => move |_, _| {
        deck.navigate(libhandy::NavigationDirection::Forward);
    }));

    credentials.connect_activate(clone!(
    @weak err_label,
    @weak server_entry,
    @weak deck => move |_, _| {
        if server_entry.get_text().is_empty() {
            err_label.show();
        } else {
            err_label.hide();
            deck.navigate(libhandy::NavigationDirection::Forward);
        }
    }));

    back.connect_activate(clone!(@weak deck => move |_, _| {
        if deck.get_adjacent_child(libhandy::NavigationDirection::Back).is_some() {
            deck.navigate(libhandy::NavigationDirection::Back);
        }
    }));

    gio::Application::get_default().map(|app| {
        app.downcast::<gtk::Application>().map(|gtk_app| {
            gtk_app.get_active_window().map(|window| {
                window.connect_button_press_event(move |_, e| {
                    if e.get_button() == 8 {
                        back.activate(None);
                    }
                    Inhibit(false)
                });
            })
        })
    });

    deck.insert_action_group("login", Some(&actions));

    actions
}
