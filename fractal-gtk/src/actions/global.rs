use glib::clone;
use log::{debug, info};
use std::convert::TryInto;

use crate::app::AppRuntime;
use crate::appop::AppOp;
use crate::model::message::Message;
use crate::util::i18n::i18n;
use crate::widgets::FileDialog::open;
use gio::prelude::*;
use gio::SimpleAction;
use gtk::prelude::*;
use libhandy::prelude::*;
use matrix_sdk::identifiers::{EventId, RoomId};

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum AppState {
    Login,
    Loading,
    NoRoom,
    Room,
    RoomSettings,
    MediaViewer,
    AccountSettings,
    Directory,
}
impl<'a> From<&'a glib::Variant> for AppState {
    fn from(v: &glib::Variant) -> AppState {
        v.get::<String>().expect("Invalid back state type").into()
    }
}

impl From<String> for AppState {
    fn from(v: String) -> AppState {
        match v.as_str() {
            "login" => AppState::Login,
            "loading" => AppState::Loading,
            "no-room" => AppState::NoRoom,
            "room" => AppState::Room,
            "media-viewer" => AppState::MediaViewer,
            "account-settings" => AppState::AccountSettings,
            "room-settings" => AppState::RoomSettings,
            "directory" => AppState::Directory,
            _ => panic!("Invalid back state type"),
        }
    }
}

impl From<AppState> for glib::Variant {
    fn from(v: AppState) -> glib::Variant {
        match v {
            AppState::Login => "login".to_variant(),
            AppState::Loading => "loading".to_variant(),
            AppState::NoRoom => "no-room".to_variant(),
            AppState::Room => "room".to_variant(),
            AppState::MediaViewer => "media-viewer".to_variant(),
            AppState::AccountSettings => "account-settings".to_variant(),
            AppState::RoomSettings => "room-setting".to_variant(),
            AppState::Directory => "directory".to_variant(),
        }
    }
}

// This creates global actions which are connected to the application
pub fn new(appop: &AppOp) {
    let app = &appop.ui.gtk_app;
    let app_runtime = appop.app_runtime.clone();

    let settings = SimpleAction::new("settings", None);
    let chat = SimpleAction::new("start_chat", None);
    let newr = SimpleAction::new("new_room", None);
    let joinr = SimpleAction::new("join_room", None);
    let logout = SimpleAction::new("logout", None);

    let inv = SimpleAction::new("room_invite", None);
    let search = SimpleAction::new("search", None);
    let leave = SimpleAction::new("leave_room", None);

    let shortcuts = SimpleAction::new("shortcuts", None);
    let about = SimpleAction::new("about", None);
    let quit = SimpleAction::new("quit", None);
    let main_menu = SimpleAction::new("main_menu", None);

    let open_room = SimpleAction::new("open-room", glib::VariantTy::new("s").ok());
    let deck_back = SimpleAction::new("deck-back", None);
    let back = SimpleAction::new("back", None);
    let media_viewer = SimpleAction::new("open-media-viewer", glib::VariantTy::new("s").ok());
    let account = SimpleAction::new("open-account-settings", None);
    let directory = SimpleAction::new("directory", None);
    //TODO: use roomid as value
    let room_settings = SimpleAction::new("open-room-settings", None);
    // TODO: send file should be a message action
    let send_file = SimpleAction::new("send-file", None);
    let send_message = SimpleAction::new("send-message", None);

    let previous_room = SimpleAction::new("previous-room", None);
    let next_room = SimpleAction::new("next-room", None);
    let prev_unread_room = SimpleAction::new("prev-unread-room", None);
    let next_unread_room = SimpleAction::new("next-unread-room", None);
    let first_room = SimpleAction::new("first-room", None);
    let last_room = SimpleAction::new("last-room", None);
    let older_messages = SimpleAction::new("older-messages", None);
    let newer_messages = SimpleAction::new("newer-messages", None);

    app.add_action(&settings);
    app.add_action(&account);
    app.add_action(&chat);
    app.add_action(&newr);
    app.add_action(&joinr);
    app.add_action(&logout);

    app.add_action(&inv);
    app.add_action(&search);
    app.add_action(&leave);

    app.add_action(&quit);
    app.add_action(&shortcuts);
    app.add_action(&about);
    app.add_action(&open_room);
    app.add_action(&deck_back);
    app.add_action(&back);
    app.add_action(&directory);
    app.add_action(&room_settings);
    app.add_action(&media_viewer);
    app.add_action(&account);
    app.add_action(&main_menu);

    app.add_action(&send_file);
    app.add_action(&send_message);

    app.add_action(&previous_room);
    app.add_action(&next_room);
    app.add_action(&prev_unread_room);
    app.add_action(&next_unread_room);
    app.add_action(&first_room);
    app.add_action(&last_room);
    app.add_action(&older_messages);
    app.add_action(&newer_messages);

    // When activated, shuts down the application
    quit.connect_activate(clone!(@weak app => move |_action, _parameter| {
        app.quit();
    }));

    about.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.about_dialog());
    }));
    main_menu.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.main_menu());
    }));

    settings.connect_activate(move |_, _| {
        info!("SETTINGS");
    });
    settings.set_enabled(false);

    logout.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.logout());
    }));
    inv.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.show_invite_user_dialog());
    }));
    chat.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.show_direct_chat_dialog());
    }));
    leave.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.leave_active_room());
    }));
    newr.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.new_room_dialog());
    }));
    joinr.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.join_to_room_dialog());
    }));

    previous_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.prev_id() {
                state.set_active_room_by_id(id);
            } else if let Some(last_room) = state.roomlist.last_id() {
                state.set_active_room_by_id(last_room);
            }
        });
    }));
    next_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.next_id() {
                state.set_active_room_by_id(id);
            } else if let Some(first_room) = state.roomlist.first_id() {
                state.set_active_room_by_id(first_room);
            }
        });
    }));
    prev_unread_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.prev_unread_id() {
                state.set_active_room_by_id(id);
            }
        });
    }));
    next_unread_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.next_unread_id() {
                state.set_active_room_by_id(id);
            }
        });
    }));
    first_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.first_id() {
                state.set_active_room_by_id(id);
            }
        });
    }));
    last_room.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(id) = state.roomlist.last_id() {
                state.set_active_room_by_id(id);
            }
        });
    }));
    older_messages.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(ref mut hist) = state.history {
                hist.page_up();
            }
        });
    }));
    newer_messages.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(ref mut hist) = state.history {
                hist.page_down();
            }
        });
    }));

    account.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            state.show_account_settings_dialog();
            state.room_back_history.borrow_mut().push(AppState::AccountSettings);
        });
    }));

    directory.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            state.set_state(AppState::Directory);
            state.room_back_history.borrow_mut().push(AppState::Directory);
        });
    }));

    /* TODO: We could pass a message to this to highlight it in the room history, might be
     * handy when opening the room from a notification */
    open_room.connect_activate(clone!(@strong app_runtime => move |_, data| {
        let data = data.cloned();
        app_runtime.update_state_with(move |state| {
            if let Some(id) = get_room_id(data.as_ref()) {
                state.set_active_room_by_id(id);
                // This does nothing if fractal is already in focus
                state.activate();
            }
            // Push a new state only if the current state is not already Room
            let push = if let Some(last) = state.room_back_history.borrow().last() {
                last != &AppState::Room
            } else {
                true
            };
            if push {
                state.room_back_history.borrow_mut().push(AppState::Room);
            }
        });
    }));

    room_settings.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            state.create_room_settings();
            state.room_back_history.borrow_mut().push(AppState::RoomSettings);
        });
    }));

    media_viewer.connect_activate(clone!(@strong app_runtime => move |_, data| {
        open_viewer(&app_runtime, data.cloned());
        app_runtime.update_state_with(|state| {
            state.room_back_history.borrow_mut().push(AppState::MediaViewer);
        });
    }));

    deck_back.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            let deck = state.deck.clone();
            if deck.get_can_swipe_back() {
                deck.navigate(libhandy::NavigationDirection::Back);
            }
        });
    }));

    back.connect_activate(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| {
            if let Some(mut mv) = state.media_viewer.borrow_mut().take() {
                mv.disconnect_signal_id();
            }

            // Remove the current state from the store
            state.room_back_history.borrow_mut().pop();
            let app_state = state.room_back_history.borrow().last().cloned();
            if let Some(app_state) = app_state {
                debug!("Go back to state {:?}", app_state);
                state.set_state(app_state);
            } else {
                // Fallback when there is no back history
                debug!("There is no state to go back to. Go back to state NoRoom");
                if state.login_data.is_some() {
                    state.set_state(AppState::NoRoom);
                }
            }
        });
    }));

    send_file.connect_activate(clone!(@weak app => move |_, _| {
        if let Some(window) = app.get_active_window() {
            if let Some(path) = open(&window, i18n("Select a file").as_str(), &[]) {
                APPOP!(attach_message, (path));
            }
        }
    }));

    send_message.connect_activate(move |_, _| {
        app_runtime.update_state_with(|state| {
            let msg_entry = state.ui.sventry.view.clone();
            if let Some(buffer) = msg_entry.get_buffer() {
                let start = buffer.get_start_iter();
                let end = buffer.get_end_iter();

                if let Some(text) = buffer.get_text(&start, &end, false) {
                    state.send_message(text.to_string());
                }

                buffer.set_text("");
            }
        });
    });

    send_message.set_enabled(false);
    let buffer = appop.ui.sventry.buffer.clone();
    buffer.connect_changed(move |buffer| {
        if 0 < buffer.get_char_count() {
            send_message.set_enabled(true);
        } else {
            send_message.set_enabled(false);
        }
    });

    /* Add Keybindings to actions */
    app.set_accels_for_action("app.quit", &["<Ctrl>Q"]);
    app.set_accels_for_action("app.previous-room", &["<Ctrl>Page_Up"]);
    app.set_accels_for_action("app.next-room", &["<Ctrl>Page_Down"]);
    app.set_accels_for_action("app.prev-unread-room", &["<Ctrl><Shift>Page_Up"]);
    app.set_accels_for_action("app.next-unread-room", &["<Ctrl><Shift>Page_Down"]);
    app.set_accels_for_action("app.first-room", &["<Ctrl>Home"]);
    app.set_accels_for_action("app.last-room", &["<Ctrl>End"]);
    app.set_accels_for_action("app.older-messages", &["Page_Up"]);
    app.set_accels_for_action("app.newer-messages", &["Page_Down"]);
    app.set_accels_for_action("app.back", &["Escape"]);
    app.set_accels_for_action("app.deck-back", &["Escape"]);
    app.set_accels_for_action("app.main_menu", &["F10"]);

    // connect mouse back button to app.back action
    if let Some(window) = app.get_active_window() {
        window.connect_button_press_event(clone!(
        @weak app
        => @default-return Inhibit(false), move |_, e| {
            if e.get_button() == 8 {
                app.lookup_action("back")
                    .expect("App did not have back action.")
                    .activate(None);
                return Inhibit(true);
            }

            Inhibit(false)
        }));
    }

    // TODO: Mark active room as read when window gets focus
    //op.lock().unwrap().mark_active_room_messages();
}

pub fn get_room_id(data: Option<&glib::Variant>) -> Option<RoomId> {
    data?.get_str().and_then(|rid| rid.try_into().ok())
}

pub fn get_event_id(data: Option<&glib::Variant>) -> Option<EventId> {
    data?.get_str().and_then(|evid| evid.try_into().ok())
}

/* TODO: get message from storage once implemented */
pub(super) fn get_message_by_id(op: &AppOp, id: &EventId) -> Option<Message> {
    let room_id = op.active_room.as_ref()?;
    op.get_message_by_id(room_id, id)
}

fn open_viewer(app_runtime: &AppRuntime, data: Option<glib::Variant>) {
    app_runtime.update_state_with(move |state| {
        if let Some(msg) = get_event_id(data.as_ref())
            .as_ref()
            .and_then(|evid| get_message_by_id(state, evid))
        {
            state.create_media_viewer(msg);
        }
    });
}

pub fn activate_action(
    app_runtime: &AppRuntime,
    action_group_name: &'static str,
    action_name: &'static str,
) {
    app_runtime.update_state_with(move |state| {
        let main_window = state
            .ui
            .builder
            .get_object::<gtk::Window>("main_window")
            .expect("Can't find main_window in ui file.");
        if let Some(action_group) = main_window.get_action_group(action_group_name) {
            action_group.activate_action(action_name, None);
        }
    });
}
