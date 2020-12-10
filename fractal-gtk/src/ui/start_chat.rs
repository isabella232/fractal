use super::UI;
use crate::app::AppRuntime;
use glib::clone;
use glib::source::Continue;
use gtk::prelude::*;
use std::sync::{Arc, Mutex};

pub struct DirectChatDialog {
    pub root: gtk::Dialog,
    pub cancel: gtk::Button,
    pub button: gtk::Button,
    pub to_chat_entry_box: gtk::Box,
    pub to_chat_entry: gtk::TextView,
    pub search_scroll: gtk::ScrolledWindow,
    pub search_box: gtk::ListBox,
}

impl DirectChatDialog {
    pub fn new(parent: &libhandy::ApplicationWindow) -> Self {
        let builder = gtk::Builder::from_resource("/org/gnome/Fractal/ui/direct_chat.ui");
        let root: gtk::Dialog = builder
            .get_object("direct_chat_dialog")
            .expect("Can't find direct_chat_dialog in ui file.");
        root.set_transient_for(Some(parent));

        Self {
            root,
            cancel: builder
                .get_object("cancel_direct_chat")
                .expect("Can't find cancel_direct_chat in ui file."),
            button: builder
                .get_object("direct_chat_button")
                .expect("Can't find direct_chat_button in ui file."),
            to_chat_entry_box: builder
                .get_object("to_chat_entry_box")
                .expect("Can't find to_chat_entry_box in ui file."),
            to_chat_entry: builder
                .get_object("to_chat_entry")
                .expect("Can't find to_chat_entry in ui file."),
            search_scroll: builder
                .get_object("direct_chat_search_scroll")
                .expect("Can't find search_scroll in ui file."),
            search_box: builder
                .get_object("direct_chat_search_box")
                .expect("Can't find direct_chat_search_box in ui file."),
        }
    }

    pub fn connect(&self, app_runtime: AppRuntime) {
        if let Some(buffer) = self.to_chat_entry.get_buffer() {
            let placeholder_tag = gtk::TextTag::new(Some("placeholder"));

            placeholder_tag.set_property_foreground_rgba(Some(&gdk::RGBA {
                red: 1.0,
                green: 1.0,
                blue: 1.0,
                alpha: 0.5,
            }));

            if let Some(tag_table) = buffer.get_tag_table() {
                tag_table.add(&placeholder_tag);
            }
        }

        // this is used to cancel the timeout and not search for every key input. We'll wait 500ms
        // without key release event to launch the search
        let source_id: Arc<Mutex<Option<glib::source::SourceId>>> = Arc::new(Mutex::new(None));
        self.to_chat_entry.connect_key_release_event(clone!(@strong app_runtime => move |entry, _| {
        {
            let mut id = source_id.lock().unwrap();
            if let Some(sid) = id.take() {
                glib::source::source_remove(sid);
            }
        }

        let sid = glib::timeout_add_local(
            500,
            clone!(
                @strong entry,
                @strong source_id,
                @strong app_runtime
                => move || {
                    if let Some(buffer) = entry.get_buffer() {
                        let start = buffer.get_start_iter();
                        let end = buffer.get_end_iter();

                        if let Some(text) =
                            buffer.get_text(&start, &end, false).map(|gstr| gstr.to_string())
                        {
                            app_runtime.update_state_with(|state| state.search_invite_user(text));
                        }
                    }

                    *(source_id.lock().unwrap()) = None;
                    Continue(false)
                }),
            );

            *(source_id.lock().unwrap()) = Some(sid);
            glib::signal::Inhibit(false)
        }));

        self.to_chat_entry.connect_focus_in_event(clone!(
            @strong self.to_chat_entry_box as to_chat_entry_box,
            @strong app_runtime
            => move |_, _| {
                to_chat_entry_box.get_style_context().add_class("message-input-focused");

                app_runtime.update_state_with(|state| state.remove_invite_user_dialog_placeholder());

                Inhibit(false)
            }
        ));

        self.to_chat_entry.connect_focus_out_event(clone!(
            @strong self.to_chat_entry_box as to_chat_entry_box,
            @strong app_runtime
            => move |_, _| {
                to_chat_entry_box.get_style_context().remove_class("message-input-focused");

                app_runtime.update_state_with(|state| state.set_invite_user_dialog_placeholder());

                Inhibit(false)
            }
        ));

        if let Some(buffer) = self.to_chat_entry.get_buffer() {
            buffer.connect_delete_range(clone!(@strong app_runtime => move |_, _, _| {
                glib::idle_add_local(clone!(@strong app_runtime => move || {
                    app_runtime.update_state_with(|state| state.detect_removed_invite());
                    Continue(false)
                }));
            }));
        }

        self.root
            .connect_delete_event(clone!(@strong app_runtime => move |_, _| {
                app_runtime.update_state_with(|state| state.ui.close_direct_chat_dialog());
                glib::signal::Inhibit(true)
            }));
        self.cancel
            .connect_clicked(clone!(@strong app_runtime => move |_| {
                app_runtime.update_state_with(|state| state.ui.close_direct_chat_dialog());
            }));
        self.button.set_sensitive(false);
        self.button.connect_clicked(move |_| {
            app_runtime.update_state_with(|state| state.start_chat());
        });
    }

    pub fn show(&self) {
        self.button.set_sensitive(false);
        self.root.present();
        self.search_scroll.hide();
    }
}

impl UI {
    pub fn show_direct_chat_dialog(&self) {
        self.direct_chat_dialog.show();
    }

    pub fn close_direct_chat_dialog(&mut self) {
        self.invite_list = vec![];
        if let Some(buffer) = self.direct_chat_dialog.to_chat_entry.get_buffer() {
            let mut start = buffer.get_start_iter();
            let mut end = buffer.get_end_iter();

            buffer.delete(&mut start, &mut end);
        }
        for ch in self.direct_chat_dialog.search_box.get_children().iter() {
            self.direct_chat_dialog.search_box.remove(ch);
        }
        self.direct_chat_dialog.search_scroll.hide();
        if let Some(buffer) = self.direct_chat_dialog.to_chat_entry.get_buffer() {
            buffer.set_text("");
        }
        self.direct_chat_dialog.root.hide();
        self.direct_chat_dialog.root.resize(300, 200);
    }
}
