mod account;
mod autocomplete;
mod direct;
mod directory;
mod headerbar;
mod invite;
mod join_room;
mod language;
mod leave_room;
mod markdown;
mod new_room;
mod roomlist_search;
mod send;
mod swipeable_widgets;

use crate::appop::AppOp;

impl AppOp {
    pub fn connect_gtk(&self) {
        let ui = &self.ui;
        let app_runtime = self.app_runtime.clone();

        headerbar::connect(ui);
        send::connect(ui, app_runtime.clone());
        markdown::connect(ui, app_runtime.clone());
        autocomplete::connect(ui, app_runtime.clone());
        language::connect(ui, app_runtime.clone());
        directory::connect(ui, app_runtime.clone());
        leave_room::connect(ui, app_runtime.clone());
        new_room::connect(ui, app_runtime.clone());
        join_room::connect(ui, app_runtime.clone());
        account::connect(ui, app_runtime.clone());
        invite::connect_dialog(ui, app_runtime.clone());
        invite::connect_user(ui, app_runtime.clone());
        direct::connect(ui, app_runtime.clone());
        roomlist_search::connect(ui, app_runtime);
        swipeable_widgets::connect(ui);
    }
}
