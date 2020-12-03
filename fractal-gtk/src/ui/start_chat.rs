use super::UI;
use gtk::prelude::*;

impl UI {
    pub fn show_direct_chat_dialog(&self) {
        let dialog = self
            .builder
            .get_object::<gtk::Dialog>("direct_chat_dialog")
            .expect("Can't find direct_chat_dialog in ui file.");
        let scroll = self
            .builder
            .get_object::<gtk::Widget>("direct_chat_search_scroll")
            .expect("Can't find direct_chat_search_scroll in ui file.");
        if let Some(btn) = self.builder.get_object::<gtk::Button>("direct_chat_button") {
            btn.set_sensitive(false)
        }
        dialog.present();
        scroll.hide();
    }

    pub fn close_direct_chat_dialog(&mut self) {
        let listbox = self
            .builder
            .get_object::<gtk::ListBox>("direct_chat_search_box")
            .expect("Can't find direct_chat_search_box in ui file.");
        let scroll = self
            .builder
            .get_object::<gtk::Widget>("direct_chat_search_scroll")
            .expect("Can't find direct_chat_search_scroll in ui file.");
        let to_chat_entry = self
            .builder
            .get_object::<gtk::TextView>("to_chat_entry")
            .expect("Can't find to_chat_entry in ui file.");
        let entry = self
            .builder
            .get_object::<gtk::TextView>("to_chat_entry")
            .expect("Can't find to_chat_entry in ui file.");
        let dialog = self
            .builder
            .get_object::<gtk::Dialog>("direct_chat_dialog")
            .expect("Can't find direct_chat_dialog in ui file.");

        self.invite_list = vec![];
        if let Some(buffer) = to_chat_entry.get_buffer() {
            let mut start = buffer.get_start_iter();
            let mut end = buffer.get_end_iter();

            buffer.delete(&mut start, &mut end);
        }
        for ch in listbox.get_children().iter() {
            listbox.remove(ch);
        }
        scroll.hide();
        if let Some(buffer) = entry.get_buffer() {
            buffer.set_text("");
        }
        dialog.hide();
        dialog.resize(300, 200);
    }
}
