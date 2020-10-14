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
        headerbar::connect(self);
        send::connect(self);
        markdown::connect(self);
        autocomplete::connect(self);
        language::connect(self);
        directory::connect(self);
        leave_room::connect(self);
        new_room::connect(self);
        join_room::connect(self);
        account::connect(self);
        invite::connect_dialog(self);
        invite::connect_user(self);
        direct::connect(self);
        roomlist_search::connect(self);
        swipeable_widgets::connect(self);
    }
}
