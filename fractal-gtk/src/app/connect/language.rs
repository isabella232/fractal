use crate::app::{self, App, RUNTIME};
use crate::backend::{room, HandleError};
use glib::object::Cast;
use gtk::prelude::*;

// The TextBufferExt alias is necessary to avoid conflict with gtk's TextBufferExt
use gspell::{CheckerExt, TextBuffer, TextBufferExt as GspellTextBufferExt};

impl App {
    pub fn connect_language(&self) {
        let textview = self.ui.sventry.view.upcast_ref::<gtk::TextView>();
        if let Some(checker) = textview
            .get_buffer()
            .and_then(|gtk_buffer| TextBuffer::get_from_gtk_text_buffer(&gtk_buffer))
            .and_then(|gs_buffer| gs_buffer.get_spell_checker())
        {
            let _signal_handler = checker.connect_property_language_notify(move |checker| {
                if let Some(lang_code) = checker
                    .get_language()
                    .and_then(|lang| lang.get_code())
                    .map(String::from)
                {
                    /*If the checker is modified by fn set_language in fractal-gtk/src/appop/room.rs
                    due to the user switching rooms, the op mutex is locked already.
                    If the checker is modified by gtk due to the user switching the language, the op mutex is unlocked. */
                    if let Ok(op) = app::get_op().try_lock() {
                        if let (Some(active_room), Some(login_data)) = (op.active_room.clone(), op.login_data.as_ref()) {
                            let session_client = login_data.session_client.clone();
                            let uid = login_data.uid.clone();
                            RUNTIME.spawn(async move {
                                let query = room::set_language(session_client, &uid, &active_room, lang_code).await;
                                if let Err(err) = query {
                                    err.handle_error();
                                }
                            });
                        }
                    }
                }
            });
        }
    }
}
