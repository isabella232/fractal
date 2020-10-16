use glib::clone;
use gtk::prelude::*;

use crate::app::AppRuntime;
use crate::uibuilder::UI;

pub fn connect(ui: &UI, app_runtime: AppRuntime) {
    let dialog = ui
        .builder
        .get_object::<gtk::Dialog>("new_room_dialog")
        .expect("Can't find new_room_dialog in ui file.");
    let cancel = ui
        .builder
        .get_object::<gtk::Button>("cancel_new_room")
        .expect("Can't find cancel_new_room in ui file.");
    let confirm = ui
        .builder
        .get_object::<gtk::Button>("new_room_button")
        .expect("Can't find new_room_button in ui file.");
    let entry = ui
        .builder
        .get_object::<gtk::Entry>("new_room_name")
        .expect("Can't find new_room_name in ui file.");
    let private = ui
        .builder
        .get_object::<gtk::ToggleButton>("private_visibility_button")
        .expect("Can't find private_visibility_button in ui file.");

    private.set_active(true);
    cancel.connect_clicked(
        clone!(@strong entry, @strong dialog, @strong private => move |_| {
            dialog.hide();
            entry.set_text("");
            private.set_active(true);
        }),
    );
    dialog.connect_delete_event(
        clone!(@strong entry, @strong dialog, @strong private => move |_, _| {
            dialog.hide();
            entry.set_text("");
            private.set_active(true);
            glib::signal::Inhibit(true)
        }),
    );

    confirm.connect_clicked(
        clone!(@strong entry, @strong dialog, @strong private, @strong app_runtime => move |_| {
            dialog.hide();
            app_runtime.update_state_with(|state| state.create_new_room());
            entry.set_text("");
            private.set_active(true);
        }),
    );

    entry.connect_activate(clone!(@strong dialog => move |entry| {
        dialog.hide();
        app_runtime.update_state_with(|state| state.create_new_room());
        entry.set_text("");
        private.set_active(true);
    }));
    entry.connect_changed(clone!(@strong confirm => move |entry| {
        confirm.set_sensitive(entry.get_buffer().get_length() > 0);
    }));
}
