use glib::clone;
use gtk::prelude::*;

use glib::source::Continue;
use std::sync::{Arc, Mutex};

use crate::app::AppRuntime;
use crate::ui::UI;

pub fn connect_dialog(ui: &UI, app_runtime: AppRuntime) {
    let dialog = ui
        .builder
        .get_object::<gtk::MessageDialog>("invite_dialog")
        .expect("Can't find invite_dialog in ui file.");
    let accept = ui
        .builder
        .get_object::<gtk::Button>("invite_accept")
        .expect("Can't find invite_accept in ui file.");
    let reject = ui
        .builder
        .get_object::<gtk::Button>("invite_reject")
        .expect("Can't find invite_reject in ui file.");

    reject.connect_clicked(clone!(@strong dialog, @strong app_runtime => move |_| {
        app_runtime.update_state_with(|state| state.accept_inv(false));
        dialog.hide();
    }));
    dialog.connect_delete_event(clone!(@strong dialog, @strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.accept_inv(false));
        dialog.hide();
        glib::signal::Inhibit(true)
    }));

    accept.connect_clicked(clone!(@strong dialog => move |_| {
        app_runtime.update_state_with(|state| state.accept_inv(true));
        dialog.hide();
    }));
}

pub fn connect_user(ui: &UI, app_runtime: AppRuntime) {
    let cancel = ui
        .builder
        .get_object::<gtk::Button>("cancel_invite")
        .expect("Can't find cancel_invite in ui file.");
    let invite = ui
        .builder
        .get_object::<gtk::Button>("invite_button")
        .expect("Can't find invite_button in ui file.");
    let invite_entry_box = ui
        .builder
        .get_object::<gtk::Box>("invite_entry_box")
        .expect("Can't find invite_entry_box in ui file.");
    let invite_entry = ui
        .builder
        .get_object::<gtk::TextView>("invite_entry")
        .expect("Can't find invite_entry in ui file.");
    let dialog = ui
        .builder
        .get_object::<gtk::Dialog>("invite_user_dialog")
        .expect("Can't find invite_user_dialog in ui file.");

    if let Some(buffer) = invite_entry.get_buffer() {
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
    invite_entry.connect_key_release_event(clone!(@strong app_runtime => move |entry, _| {
        {
            let mut id = source_id.lock().unwrap();
            if let Some(sid) = id.take() {
                glib::source::source_remove(sid);
            }
        }

        let sid = glib::timeout_add_local(
            500,
            clone!(@strong entry, @strong source_id, @strong app_runtime => move || {
                if let Some(buffer) = entry.get_buffer() {
                    let start = buffer.get_start_iter();
                    let end = buffer.get_end_iter();

                    if let Some(text) = buffer.get_text(&start, &end, false).map(|gstr| gstr.to_string()) {
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

    invite_entry.connect_focus_in_event(
        clone!(@strong invite_entry_box, @strong app_runtime => move |_, _| {
            invite_entry_box.get_style_context().add_class("message-input-focused");

            app_runtime.update_state_with(|state| state.remove_invite_user_dialog_placeholder());

            Inhibit(false)
        }),
    );

    invite_entry.connect_focus_out_event(
        clone!(@strong invite_entry_box, @strong app_runtime => move |_, _| {
            invite_entry_box.get_style_context().remove_class("message-input-focused");

            app_runtime.update_state_with(|state| state.set_invite_user_dialog_placeholder());

            Inhibit(false)
        }),
    );

    if let Some(buffer) = invite_entry.get_buffer() {
        buffer.connect_delete_range(clone!(@strong app_runtime => move |_, _, _| {
            glib::idle_add_local(clone!(@strong app_runtime => move || {
                app_runtime.update_state_with(|state| state.detect_removed_invite());
                Continue(false)
            }));
        }));
    }

    dialog.connect_delete_event(clone!(@strong app_runtime => move |_, _| {
        app_runtime.update_state_with(|state| state.ui.close_invite_dialog());
        glib::signal::Inhibit(true)
    }));
    cancel.connect_clicked(clone!(@strong app_runtime => move |_| {
        app_runtime.update_state_with(|state| state.ui.close_invite_dialog());
    }));
    invite.set_sensitive(false);
    invite.connect_clicked(move |_| {
        app_runtime.update_state_with(|state| state.invite());
    });
}
