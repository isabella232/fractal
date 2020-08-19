use crate::backend::{dw_media, media, room, ContentType};
use fractal_api::identifiers::RoomId;
use fractal_api::r0::AccessToken;
use fractal_api::Client as MatrixClient;
use glib::clone;
use log::error;
use std::cell::RefCell;
use std::fs;
use std::process::Command;
use std::rc::Rc;
use std::thread;

use crate::actions::AppState;
use crate::app::RUNTIME;
use crate::backend::HandleError;
use crate::model::message::Message;
use crate::uibuilder::UI;
use crate::util::i18n::i18n;
use crate::App;
use gio::ActionGroupExt;
use gio::ActionMapExt;
use gio::SimpleAction;
use gio::SimpleActionGroup;
use gtk::prelude::*;

use super::global::{get_event_id, get_message_by_id, get_room_id};

use crate::widgets::ErrorDialog;
use crate::widgets::FileDialog::save;
use crate::widgets::SourceDialog;

/* This creates all actions the room history can perform */
pub fn new(
    session_client: MatrixClient,
    access_token: AccessToken,
    ui: UI,
    back_history: Rc<RefCell<Vec<AppState>>>,
) -> gio::SimpleActionGroup {
    let server_url = session_client.homeserver().clone();
    let actions = SimpleActionGroup::new();
    /* Action for each message */
    let reply = SimpleAction::new("reply", glib::VariantTy::new("s").ok());
    let open_with = SimpleAction::new("open_with", glib::VariantTy::new("s").ok());
    let save_as = SimpleAction::new("save_as", glib::VariantTy::new("s").ok());
    let copy_image = SimpleAction::new("copy_image", glib::VariantTy::new("s").ok());
    let copy_text = SimpleAction::new("copy_text", glib::VariantTy::new("s").ok());
    let delete = SimpleAction::new("delete", glib::VariantTy::new("s").ok());
    let show_source = SimpleAction::new("show_source", glib::VariantTy::new("s").ok());

    /* TODO: use stateful action to keep  track if the user already requested new messages */
    let load_more_messages =
        SimpleAction::new("request_older_messages", glib::VariantTy::new("s").ok());

    actions.add_action(&reply);
    actions.add_action(&open_with);
    actions.add_action(&save_as);
    actions.add_action(&copy_image);
    actions.add_action(&copy_text);
    actions.add_action(&delete);
    actions.add_action(&show_source);
    actions.add_action(&load_more_messages);

    let parent: gtk::Window = ui
        .builder
        .get_object("main_window")
        .expect("Can't find main_window in ui file.");
    show_source.connect_activate(clone!(@weak parent => move |_, data| {
        let viewer = SourceDialog::new();
        viewer.set_parent_window(&parent);
        if let Some(m) = get_message(data) {
            let error = i18n("This message has no source.");
            let source = m.source.as_ref().unwrap_or(&error);

            viewer.show(source);
        }
    }));

    let window = ui
        .builder
        .get_object::<gtk::ApplicationWindow>("main_window")
        .expect("Couldn't find main_window in ui file.");
    reply.connect_activate(clone!(
    @weak back_history,
    @weak window,
    @weak ui.sventry.view as msg_entry
    => move |_, data| {
        let state = back_history.borrow().last().cloned();
        if let Some(AppState::MediaViewer) = state {
            if let Some(action_group) = window.get_action_group("app") {
                action_group.activate_action("back", None);
            } else {
                error!("The action group app is not attached to the main window.");
            }
        }
        if let Some(buffer) = msg_entry.get_buffer() {
            let mut start = buffer.get_start_iter();
            if let Some(m) = get_message(data) {
                let quote = m
                    .body
                    .lines()
                    .map(|l| "> ".to_owned() + l)
                    .collect::<Vec<String>>()
                    .join("\n")
                    + "\n"
                    + "\n";
                buffer.insert(&mut start, &quote);
                msg_entry.grab_focus();
            }
        }
    }));

    open_with.connect_activate(clone!(@strong session_client => move |_, data| {
        if let Some(url) = get_message(data).and_then(|m| m.url) {
            let session_client = session_client.clone();
            RUNTIME.spawn(async move {
                match dw_media(session_client, &url, ContentType::Download, None).await {
                    Ok(fname) => {
                        Command::new("xdg-open")
                            .arg(&fname)
                            .spawn()
                            .expect("failed to execute process");
                    }
                    Err(err) => {
                        err.handle_error()
                    }
                }
            });
        }
    }));

    save_as.connect_activate(clone!(
    @strong session_client,
    @weak parent as window
    => move |_, data| {
        if let Some((Some(url), name)) = get_message(data).map(|m| (m.url, m.body)) {
            let session_client = session_client.clone();
            let response = RUNTIME.spawn(async move {
                media::get_media(session_client, &url).await
            });

            glib::MainContext::default().spawn_local(async move {
                match response.await {
                    Err(_) => {
                        let msg = i18n("Could not download the file");
                        ErrorDialog::new(false, &msg);
                    },
                    Ok(Ok(fname)) => {
                        if let Some(path) = save(&window.upcast_ref(), &name, &[]) {
                            // TODO use glib to copy file
                            if fs::copy(fname, path).is_err() {
                                ErrorDialog::new(false, &i18n("Couldn’t save file"));
                            }
                        }
                    }
                    Ok(Err(err)) => {
                        error!("Media path could not be found due to error: {:?}", err);
                    }
                }
            });
        }
    }));

    copy_image.connect_activate(clone!(@strong session_client => move |_, data| {
        if let Some(url) = get_message(data).and_then(|m| m.url) {
            let session_client = session_client.clone();
            let response = RUNTIME.spawn(async move {
                media::get_media(session_client, &url).await
            });

            glib::MainContext::default().spawn_local(async move {
                match response.await {
                    Err(_) => {
                        let msg = i18n("Could not download the file");
                        ErrorDialog::new(false, &msg);
                    }
                    Ok(Ok(fname)) => {
                        if let Ok(pixbuf) = gdk_pixbuf::Pixbuf::from_file(fname) {
                            let atom = gdk::Atom::intern("CLIPBOARD");
                            let clipboard = gtk::Clipboard::get(&atom);
                            clipboard.set_image(&pixbuf);
                        }
                    }
                    Ok(Err(err)) => {
                        error!("Image path could not be found due to error: {:?}", err);
                    }
                }
            });
        }
    }));

    copy_text.connect_activate(move |_, data| {
        if let Some(m) = get_message(data) {
            let atom = gdk::Atom::intern("CLIPBOARD");
            let clipboard = gtk::Clipboard::get(&atom);

            clipboard.set_text(&m.body);
        }
    });

    let s = server_url.clone();
    let tk = access_token.clone();
    delete.connect_activate(clone!(
    @weak back_history,
    @weak window
    => move |_, data| {
        let state = back_history.borrow().last().cloned();
        if let Some(AppState::MediaViewer) = state {
            if let Some(action_group) = window.get_action_group("app") {
                action_group.activate_action("back", None);
            } else {
                error!("The action group app is not attached to the main window.");
            }
        }
        if let Some(msg) = get_message(data) {
            let server = s.clone();
            let access_token = tk.clone();
            thread::spawn(move || {
                let query = room::redact_msg(server, access_token, msg);
                if let Err(err) = query {
                    err.handle_error();
                }
            });
        }
    }));

    load_more_messages.connect_activate(move |_, data| {
        let id = get_room_id(data);
        request_more_messages(session_client.clone(), id);
    });

    actions
}

fn get_message(id: Option<&glib::Variant>) -> Option<Message> {
    get_event_id(id).as_ref().and_then(get_message_by_id)
}

fn request_more_messages(session_client: MatrixClient, id: Option<RoomId>) -> Option<()> {
    let op = App::get_op()?;
    let op = op.lock().unwrap();
    let id = id?;
    let r = op.rooms.get(&id)?;
    if let Some(prev_batch) = r.prev_batch.clone() {
        RUNTIME.spawn(async move {
            match room::get_room_messages(session_client, id, &prev_batch).await {
                Ok((msgs, room, prev_batch)) => {
                    APPOP!(show_room_messages_top, (msgs, room, prev_batch));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    } else if let Some(msg) = r.messages.iter().next().cloned() {
        // no prev_batch so we use the last message to calculate that in the backend
        RUNTIME.spawn(async move {
            match room::get_room_messages_from_msg(session_client, id, msg).await {
                Ok((msgs, room, prev_batch)) => {
                    APPOP!(show_room_messages_top, (msgs, room, prev_batch));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    } else if let Some(from) = op.since.clone() {
        // no messages and no prev_batch so we use the last since
        RUNTIME.spawn(async move {
            match room::get_room_messages(session_client, id, &from).await {
                Ok((msgs, room, prev_batch)) => {
                    APPOP!(show_room_messages_top, (msgs, room, prev_batch));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }
    None
}
