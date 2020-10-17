use crate::util::i18n::i18n;
use crate::widgets::SVEntry;
use gtk::{self, prelude::*};

#[derive(Clone, Debug)]
pub struct UI {
    pub builder: gtk::Builder,
    pub gtk_app: gtk::Application,
    pub main_window: libhandy::ApplicationWindow,
    pub sventry: SVEntry,
    pub sventry_box: Box<gtk::Stack>,
}

impl UI {
    pub fn new(gtk_app: gtk::Application) -> UI {
        // The order here is important because some ui file depends on others

        let builder = gtk::Builder::new();

        builder
            .add_from_resource("/org/gnome/Fractal/ui/autocomplete.ui")
            .expect("Can't load ui file: autocomplete.ui");

        // needed from main_window
        // These are popup menus showed from main_window interface
        builder
            .add_from_resource("/org/gnome/Fractal/ui/main_menu.ui")
            .expect("Can't load ui file: main_menu.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/add_room_menu.ui")
            .expect("Can't load ui file: add_room_menu.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/room_menu.ui")
            .expect("Can't load ui file: room_menu.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/markdown_popover.ui")
            .expect("Can't load ui file: markdown_popover.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/server_chooser_menu.ui")
            .expect("Can't load ui file: server_chooser_menu.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/main_window.ui")
            .expect("Can't load ui file: main_window.ui");

        // Order which sventry is created matters
        let sventry_stack = gtk::Stack::new();

        let sventry = SVEntry::default();
        sventry_stack.add_named(&sventry.clamp, "Text Entry");
        let sventry_disabled = gtk::Label::new(Some(&i18n(
            "You don’t have permission to post to this room",
        )));
        sventry_disabled.set_hexpand(false);
        sventry_disabled.get_style_context().add_class("dim-label");
        sventry_disabled.set_line_wrap(true);
        sventry_disabled.set_line_wrap_mode(pango::WrapMode::WordChar);
        sventry_stack.add_named(&sventry_disabled, "Disabled Entry");

        let sventry_box = Box::new(sventry_stack.clone());
        let parent: gtk::Box = builder.get_object("room_parent").unwrap();
        parent.add(&sventry_stack);

        // Depends on main_window
        // These are all dialogs transient for main_window
        builder
            .add_from_resource("/org/gnome/Fractal/ui/direct_chat.ui")
            .expect("Can't load ui file: direct_chat.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/invite.ui")
            .expect("Can't load ui file: invite.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/invite_user.ui")
            .expect("Can't load ui file: invite_user.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/join_room.ui")
            .expect("Can't load ui file: join_room.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/leave_room.ui")
            .expect("Can't load ui file: leave_room.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/new_room.ui")
            .expect("Can't load ui file: new_room.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/password_dialog.ui")
            .expect("Can't load ui file: password_dialog.ui");
        builder
            .add_from_resource("/org/gnome/Fractal/ui/account_settings.ui")
            .expect("Can't load ui file: account_settings.ui");

        let main_window: libhandy::ApplicationWindow = builder
            .get_object("main_window")
            .expect("Couldn't find main_window in ui file.");
        main_window.set_application(Some(&gtk_app));
        main_window.set_title("Fractal");

        UI {
            builder,
            gtk_app,
            main_window,
            sventry,
            sventry_box,
        }
    }
}
