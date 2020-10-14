use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::backend::{room, HandleError};
use glib::object::Cast;
use gtk::prelude::*;

// The TextBufferExt alias is necessary to avoid conflict with gtk's TextBufferExt
use gspell::{CheckerExt, TextBuffer, TextBufferExt as GspellTextBufferExt};

pub fn connect(appop: &AppOp) {
    let app_tx = appop.app_tx.clone();
    let textview = appop.ui.sventry.view.upcast_ref::<gtk::TextView>();
    if let Some(checker) = textview
        .get_buffer()
        .and_then(|gtk_buffer| TextBuffer::get_from_gtk_text_buffer(&gtk_buffer))
        .and_then(|gs_buffer| gs_buffer.get_spell_checker())
    {
        let _signal_handler = checker.connect_property_language_notify(move |checker| {
                let _ = app_tx.send(Box::new(clone!(@weak checker => move |op| {
                    if let Some(lang_code) = checker
                        .get_language()
                        .and_then(|lang| lang.get_code())
                        .map(String::from)
                    {
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
                })));
            });
    }
}
