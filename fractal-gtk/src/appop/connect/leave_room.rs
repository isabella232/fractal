use glib::clone;
use gtk::prelude::*;

use crate::appop::AppOp;

pub fn connect(appop: &AppOp) {
    let app_tx = appop.app_tx.clone();
    let dialog = appop
        .ui
        .builder
        .get_object::<gtk::Dialog>("leave_room_dialog")
        .expect("Can't find leave_room_dialog in ui file.");
    let cancel = appop
        .ui
        .builder
        .get_object::<gtk::Button>("leave_room_cancel")
        .expect("Can't find leave_room_cancel in ui file.");
    let confirm = appop
        .ui
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
        let _ = app_tx.send(Box::new(|op| op.really_leave_active_room()));
    }));
}
