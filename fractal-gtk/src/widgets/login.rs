use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use libhandy::prelude::*;
use log::info;
use url::Url;

use crate::actions;
use crate::actions::global::AppState;
use crate::actions::login::LoginState;
use crate::app::{AppRuntime, RUNTIME};
use crate::globals;
use crate::util::i18n::i18n;
use crate::widgets::ErrorDialog;

use crate::backend::register::get_well_known;

use std::convert::TryInto;

#[derive(Debug, Clone)]
pub struct LoginWidget {
    pub container: libhandy::Deck,
    pub server_entry: gtk::Entry,
    pub username_entry: gtk::Entry,
    pub password_entry: gtk::Entry,
    server_err_label: gtk::Label,
    credentials_err_label: gtk::Label,
    actions: gio::SimpleActionGroup,
}

impl LoginWidget {
    pub fn new(app_runtime: AppRuntime) -> Self {
        let widget = Self::default();

        let server_entry = &widget.server_entry;
        let username_entry = &widget.username_entry;
        let password_entry = &widget.password_entry;
        let err_label = &widget.credentials_err_label;

        // Grab the focus for each state
        widget
            .container
            .connect_property_visible_child_name_notify(clone!(
            @weak server_entry as server,
            @weak username_entry as username
            => move |container| {
                let state: LoginState = container
                    .get_visible_child_name()
                    .unwrap()
                    .to_string()
                    .into();

                match state {
                    LoginState::ServerChooser => server.grab_focus(),
                    LoginState::Credentials => username.grab_focus(),
                    _ => (),
                }
            }));

        let login = widget
            .actions
            .lookup_action("login")
            .expect("Could not find 'login' action for LoginWidget")
            .downcast::<gio::SimpleAction>()
            .expect("Could not cast action 'login' to SimpleAction");

        login.connect_activate(clone!(
        @weak server_entry,
        @weak username_entry,
        @weak password_entry,
        @weak err_label
        => move |_, _| {
            let username = username_entry
                .get_text()
                .to_string();

            let password = password_entry
                .get_text()
                .to_string();

            let txt = server_entry.get_text().to_string().trim().to_string();
            let txt = if txt.starts_with("http://") || txt.starts_with("https://") {
                txt
            } else {
                format!("https://{}", &txt)
            };
            let txt = if !txt.ends_with('/') { txt + "/" } else { txt };

            if !password.is_empty() && !username.is_empty() {
                // take the user's homeserver value if the
                // well-known request fails
                let homeserver_url = if let Ok(hs_url) = Url::parse(&txt) {
                    hs_url
                } else {
                    let msg = i18n("Malformed server URL");
                    ErrorDialog::new(false, &msg);
                    return;
                };

                let query = get_well_known(homeserver_url.clone());
                let (homeserver_url, idserver) = RUNTIME.handle().block_on(query)
                    .and_then(|response| {
                        let hs_url = Url::parse(&response.homeserver.base_url)?;
                        let ids = response
                            .identity_server
                            .as_ref()
                            .map(|ids| Url::parse(&ids.base_url))
                            .transpose()?
                            .as_ref()
                            .and_then(Url::host_str)
                            .map(TryInto::try_into)
                            .transpose()?
                            .unwrap_or(globals::DEFAULT_IDENTITYSERVER.clone());
                        info!("Got well-known response from {}: {:#?}", &txt, response);

                        Ok((hs_url, ids))
                    })
                    .map_err(|e| {
                        info!("Failed to .well-known request: {:#?}", e);
                        e
                    })
                    .unwrap_or((homeserver_url, globals::DEFAULT_IDENTITYSERVER.clone()));

                err_label.hide();
                app_runtime.update_state_with(|state| {
                    state.set_state(AppState::Loading);
                    state.since = None;
                    state.connect(username, password, homeserver_url, idserver);
                });
            } else {
                err_label.show();
            }
        }));

        let credentials = widget
            .actions
            .lookup_action("credentials")
            .expect("Could not find 'credentials' action for LoginWidget")
            .downcast::<gio::SimpleAction>()
            .expect("Could not cast action 'credentials' to SimpleAction");
        widget
            .server_entry
            .connect_activate(move |_| credentials.activate(None));

        widget
            .username_entry
            .connect_activate(clone!(@weak password_entry => move |_| {
                password_entry.grab_focus();
            }));

        widget
            .password_entry
            .connect_activate(move |_| login.activate(None));

        widget
    }
}

impl Default for LoginWidget {
    fn default() -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Fractal/ui/login_flow.ui");

        let container: libhandy::Deck = builder.get_object("login_flow_deck").unwrap();
        let server_entry = builder.get_object("server_chooser_entry").unwrap();
        let username_entry = builder.get_object("username_entry").unwrap();
        let password_entry = builder.get_object("password_entry").unwrap();

        let server_err_label = builder.get_object("server_err_label").unwrap();
        let credentials_err_label = builder.get_object("credentials_err_label").unwrap();

        let actions = actions::Login::new(&container, &server_entry, &server_err_label);

        container.show_all();

        LoginWidget {
            container,
            server_entry,
            username_entry,
            password_entry,
            server_err_label,
            credentials_err_label,
            actions,
        }
    }
}
