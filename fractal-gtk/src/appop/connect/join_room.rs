use glib::clone;
use gtk::prelude::*;

use crate::appop::AppOp;

pub fn connect(appop: &AppOp) {
    let app_runtime = appop.app_runtime.clone();
    let dialog = appop
        .ui
        .builder
        .get_object::<gtk::Dialog>("join_room_dialog")
        .expect("Can't find join_room_dialog in ui file.");
    let cancel = appop
        .ui
        .builder
        .get_object::<gtk::Button>("cancel_join_room")
        .expect("Can't find cancel_join_room in ui file.");
    let confirm = appop
        .ui
        .builder
        .get_object::<gtk::Button>("join_room_button")
        .expect("Can't find join_room_button in ui file.");
    let entry = appop
        .ui
        .builder
        .get_object::<gtk::Entry>("join_room_name")
        .expect("Can't find join_room_name in ui file.");

    cancel.connect_clicked(clone!(@strong entry, @strong dialog => move |_| {
        dialog.hide();
        entry.set_text("");
    }));
    dialog.connect_delete_event(clone!(@strong entry, @strong dialog => move |_, _| {
        dialog.hide();
        entry.set_text("");
        glib::signal::Inhibit(true)
    }));

    confirm.connect_clicked(
        clone!(@strong entry, @strong dialog, @strong app_runtime => move |_| {
            dialog.hide();
            app_runtime.update_state_with(|state| state.join_to_room());
            entry.set_text("");
        }),
    );

    entry.connect_activate(clone!(@strong dialog => move |entry| {
        dialog.hide();
        app_runtime.update_state_with(|state| state.join_to_room());
        entry.set_text("");
    }));
    entry.connect_changed(clone!(@strong confirm => move |entry| {
            confirm.set_sensitive(entry.get_buffer().get_length() > 0);
    }));
}
