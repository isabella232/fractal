extern crate gtk;
use self::gtk::prelude::*;

use std::sync::{Arc, Mutex};
use glib;

use app::App;

impl App {
    pub fn connect_invite_dialog(&self) {
        let op = self.op.clone();
        let dialog = self.ui.builder
            .get_object::<gtk::MessageDialog>("invite_dialog")
            .expect("Can't find invite_dialog in ui file.");
        let accept = self.ui.builder
            .get_object::<gtk::Button>("invite_accept")
            .expect("Can't find invite_accept in ui file.");
        let reject = self.ui.builder
            .get_object::<gtk::Button>("invite_reject")
            .expect("Can't find invite_reject in ui file.");

        reject.connect_clicked(clone!(dialog, op => move |_| {
            op.lock().unwrap().accept_inv(false);
            dialog.hide();
        }));
        dialog.connect_delete_event(clone!(dialog, op => move |_, _| {
            op.lock().unwrap().accept_inv(false);
            dialog.hide();
            glib::signal::Inhibit(true)
        }));

        accept.connect_clicked(clone!(dialog, op => move |_| {
            op.lock().unwrap().accept_inv(true);
            dialog.hide();
        }));
    }

    pub fn connect_invite_user(&self) {
        let op = &self.op;

        let cancel = self.ui.builder
            .get_object::<gtk::Button>("cancel_invite")
            .expect("Can't find cancel_invite in ui file.");
        let invite = self.ui.builder
            .get_object::<gtk::Button>("invite_button")
            .expect("Can't find invite_button in ui file.");
        let to_chat_textview = self.ui.builder
            .get_object::<gtk::TextView>("to_chat_textview")
            .expect("Can't find to_chat_textview in ui file.");
        let invite_textview = self.ui.builder
            .get_object::<gtk::TextView>("invite_textview")
            .expect("Can't find invite_textview in ui file.");
        let entry = self.ui.builder
            .get_object::<gtk::Entry>("invite_entry")
            .expect("Can't find invite_entry in ui file.");
        let dialog = self.ui.builder
            .get_object::<gtk::Dialog>("invite_user_dialog")
            .expect("Can't find invite_user_dialog in ui file.");

        // this is used to cancel the timeout and not search for every key input. We'll wait 500ms
        // without key release event to launch the search
        let source_id: Arc<Mutex<Option<glib::source::SourceId>>> = Arc::new(Mutex::new(None));
        entry.connect_key_release_event(clone!(op => move |entry, _| {
            {
                let mut id = source_id.lock().unwrap();
                if let Some(sid) = id.take() {
                    glib::source::source_remove(sid);
                }
            }

            let sid = gtk::timeout_add(500, clone!(op, entry, source_id => move || {
                op.lock().unwrap().search_invite_user(entry.get_text());
                *(source_id.lock().unwrap()) = None;
                gtk::Continue(false)
            }));

            *(source_id.lock().unwrap()) = Some(sid);
            glib::signal::Inhibit(false)
        }));

        if let Some(buffer) = to_chat_textview.get_buffer() {
            buffer.connect_delete_range(clone!( op => move |_, _, _| {
                gtk::idle_add(clone!(op => move || {
                    op.lock().unwrap().detect_removed_invite();
                    Continue(false)
                }));
            }));
        }

        if let Some(buffer) = invite_textview.get_buffer() {
            buffer.connect_delete_range(clone!( op => move |_, _, _| {
                gtk::idle_add(clone!(op => move || {
                    op.lock().unwrap().detect_removed_invite();
                    Continue(false)
                }));
            }));
        }

        dialog.connect_delete_event(clone!(op => move |_, _| {
            op.lock().unwrap().close_invite_dialog();
            glib::signal::Inhibit(true)
        }));
        cancel.connect_clicked(clone!(op => move |_| {
            op.lock().unwrap().close_invite_dialog();
        }));
        invite.set_sensitive(false);
        invite.connect_clicked(clone!(op => move |_| {
            op.lock().unwrap().invite();
        }));
    }
}
