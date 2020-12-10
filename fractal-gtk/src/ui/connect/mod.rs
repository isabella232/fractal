mod autocomplete;
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

use crate::app::AppRuntime;
use crate::ui::UI;

impl UI {
    pub fn connect_gtk(&self, app_runtime: AppRuntime) {
        headerbar::connect(self);
        send::connect(self, app_runtime.clone());
        markdown::connect(self, app_runtime.clone());
        autocomplete::connect(self, app_runtime.clone());
        language::connect(self, app_runtime.clone());
        directory::connect(self, app_runtime.clone());
        leave_room::connect(self, app_runtime.clone());
        new_room::connect(self, app_runtime.clone());
        join_room::connect(self, app_runtime.clone());
        self.account_settings
            .connect(&self.builder, &self.main_window, app_runtime.clone());
        invite::connect_dialog(self, app_runtime.clone());
        invite::connect_user(self, app_runtime.clone());
        self.direct_chat_dialog.connect(app_runtime.clone());
        roomlist_search::connect(self, app_runtime);
        swipeable_widgets::connect(self);
    }
}
