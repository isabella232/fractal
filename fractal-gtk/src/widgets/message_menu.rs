extern crate gdk;
extern crate gtk;
extern crate sourceview;

use self::gtk::prelude::*;
use self::sourceview::prelude::*;

use types::Message;

use uibuilder::UI;

#[derive(Clone)]
pub struct MessageMenu {
    ui: UI,
    pub msg: Message,
}

impl MessageMenu {
    pub fn new_message_menu(ui: UI, msg: Message) -> MessageMenu {
        MessageMenu {
            ui,
            msg,
        }
    }

    pub fn show_menu_popover(&self, w: gtk::Widget, (x, y): (f64, f64)) {
        let menu_popover: gtk::Popover = self.ui.builder
            .get_object("message_menu_popover")
            .expect("Can't find message_menu_popover in ui file.");
        let rect = gtk::Rectangle {
            x: x as i32,
            y: y as i32,
            width: 0,
            height: 0,
        };

        menu_popover.set_relative_to(&w);
        menu_popover.set_pointing_to(&rect);
        menu_popover.set_position(gtk::PositionType::Bottom);

        menu_popover.popup();
    }

    pub fn insert_quote(&self) {
        let msg_entry: sourceview::View = self.ui.builder
            .get_object("msg_entry")
            .expect("Can't find msg_entry in ui file.");

        if let Some(buffer) = msg_entry.get_buffer() {
            let quote = self.msg.body.lines().map(|l| "> ".to_owned() + l + "\n")
                                .collect::<Vec<String>>().join("\n") + "\n";

            let mut start = buffer.get_start_iter();
            buffer.insert(&mut start, &quote);

            msg_entry.grab_focus();
        }
    }

    pub fn copy_text(&self) {
        let atom = gdk::Atom::intern("CLIPBOARD");
        let clipboard = gtk::Clipboard::get(&atom);

        clipboard.set_text(&self.msg.body);
    }

    pub fn display_source_dialog(&self) {
        let dialog: gtk::MessageDialog = self.ui.builder
            .get_object("source_dialog")
            .expect("Can't find source_dialog in ui file.");

        let json_lang = sourceview::LanguageManager::get_default()
                                                   .map_or(None, |lm| lm.get_language("json"));

        let buffer: sourceview::Buffer = self.ui.builder
            .get_object("msg_source_buffer")
            .expect("Can't find msg_source_buffer in ui file.");
        buffer.set_highlight_matching_brackets(false);
        if let Some(json_lang) = json_lang.clone() {
            buffer.set_language(&json_lang);
            buffer.set_highlight_syntax(true);
        }

        buffer.set_text(self.msg.source.clone()
                                       .unwrap_or("This message has no source.".to_string())
                                       .as_str());

        dialog.connect_response(move |d, res| {
            if gtk::ResponseType::from(res) == gtk::ResponseType::Accept {
                let atom = gdk::Atom::intern("CLIPBOARD");
                let clipboard = gtk::Clipboard::get(&atom);

                let start_iter = buffer.get_start_iter();
                let end_iter = buffer.get_end_iter();

                if let Some(src) = buffer.get_text(&start_iter, &end_iter, false) {
                    clipboard.set_text(&src);
                }
            } else {
                d.hide();
            }
        });

        dialog.show();
    }
}
