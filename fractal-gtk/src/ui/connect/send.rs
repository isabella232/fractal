use glib::clone;
use gtk::prelude::*;
use sourceview4::BufferExt;

use crate::actions::activate_action;
use crate::app::AppRuntime;
use crate::ui::UI;

const MAX_INPUT_HEIGHT: i32 = 100;

pub fn connect(ui: &UI, app_runtime: AppRuntime) {
    ui.sventry.container.set_redraw_on_allocate(true);
    let msg_entry = ui.sventry.view.clone();
    let buffer = &ui.sventry.buffer;
    buffer.set_highlight_matching_brackets(false);

    let msg_entry_box = ui.sventry.entry_box.clone();
    msg_entry_box.set_redraw_on_allocate(true);

    if let Some(adjustment) = ui.sventry.scroll.get_vadjustment() {
        adjustment.connect_value_changed(clone!(@strong msg_entry => move |adj| {
            if msg_entry.get_allocated_height() < MAX_INPUT_HEIGHT {
                adj.set_value(0.0);
            }
        }));
    }

    let autocomplete_popover = ui
        .builder
        .get_object::<gtk::Popover>("autocomplete_popover")
        .expect("Can't find autocomplete_popover in ui file.");

    msg_entry.connect_key_press_event(
        clone!(@strong app_runtime => move |_, key| match key.get_keyval() {
            gdk::keys::constants::Return | gdk::keys::constants::KP_Enter
                if !key.get_state().contains(gdk::ModifierType::SHIFT_MASK)
                    && !autocomplete_popover.is_visible() =>
            {
                activate_action(&app_runtime, "app", "send-message");
                Inhibit(true)
            }
            _ => Inhibit(false),
        }),
    );

    msg_entry.connect_key_release_event(clone!(@strong app_runtime => move |_, ev| {
        if ev.get_keyval().to_unicode().is_some() {
            app_runtime.update_state_with(|state| state.send_typing());
        }
        Inhibit(false)
    }));

    msg_entry.connect_paste_clipboard(move |_| {
        app_runtime.update_state_with(|state| state.paste());
    });

    msg_entry.connect_focus_in_event(clone!(@strong msg_entry_box => move |_, _| {
        msg_entry_box.get_style_context().add_class("message-input-focused");

        Inhibit(false)
    }));

    msg_entry.connect_focus_out_event(clone!(@strong msg_entry_box => move |_, _| {
        msg_entry_box.get_style_context().remove_class("message-input-focused");

        Inhibit(false)
    }));
}
