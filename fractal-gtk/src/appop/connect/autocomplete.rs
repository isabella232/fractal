use gtk::prelude::*;

use crate::widgets;

use crate::appop::AppOp;

pub fn connect(appop: &AppOp) {
    let popover = appop
        .ui
        .builder
        .get_object::<gtk::Popover>("autocomplete_popover")
        .expect("Can't find autocomplete_popover in ui file.");
    let listbox = appop
        .ui
        .builder
        .get_object::<gtk::ListBox>("autocomplete_listbox")
        .expect("Can't find autocomplete_listbox in ui file.");
    let window: gtk::Window = appop
        .ui
        .builder
        .get_object("main_window")
        .expect("Can't find main_window in ui file.");

    widgets::Autocomplete::new(window, appop.ui.sventry.view.clone(), popover, listbox).connect();
}
