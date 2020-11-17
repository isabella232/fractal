use crate::app::{AppRuntime, RUNTIME};
use crate::backend::{room, HandleError};
use crate::ui::UI;
use glib::object::Cast;
use gtk::prelude::*;

// The TextBufferExt alias is necessary to avoid conflict with gtk's TextBufferExt
use gspell::{CheckerExt, TextBuffer, TextBufferExt as GspellTextBufferExt};

pub fn connect(ui: &UI, app_runtime: AppRuntime) {
    let textview = ui.sventry.view.upcast_ref::<gtk::TextView>();
    if let Some(checker) = textview
        .get_buffer()
        .and_then(|gtk_buffer| TextBuffer::get_from_gtk_text_buffer(&gtk_buffer))
        .and_then(|gs_buffer| gs_buffer.get_spell_checker())
    {
        let _signal_handler = checker.connect_property_language_notify(move |checker| {
                app_runtime.update_state_with(clone!(@weak checker => move |state| {
                    if let Some(lang_code) = checker
                        .get_language()
                        .and_then(|lang| lang.get_code())
                        .map(String::from)
                    {
                        if let (Some(active_room), Some(login_data)) = (state.active_room.clone(), state.login_data.as_ref()) {
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
                }));
            });
    }
}
