use glib::clone;
use gtk::prelude::*;

use crate::app::AppRuntime;
use crate::uibuilder::UI;

pub fn connect(ui: &UI, app_runtime: AppRuntime) {
    let dialog = ui
        .builder
        .get_object::<gtk::Dialog>("leave_room_dialog")
        .expect("Can't find leave_room_dialog in ui file.");
    let cancel = ui
        .builder
        .get_object::<gtk::Button>("leave_room_cancel")
        .expect("Can't find leave_room_cancel in ui file.");
    let confirm = ui
        .builder
        .get_object::<gtk::Button>("leave_room_confirm")
        .expect("Can't find leave_room_confirm in ui file.");

    cancel.connect_clicked(clone!(@strong dialog => move |_| {
        dialog.hide();
    }));
    dialog.connect_delete_event(clone!(@strong dialog => move |_, _| {
        dialog.hide();
        glib::signal::Inhibit(true)
    }));

    confirm.connect_clicked(clone!(@strong dialog => move |_| {
        dialog.hide();
        app_runtime.update_state_with(move |state| state.really_leave_active_room());
    }));
}
