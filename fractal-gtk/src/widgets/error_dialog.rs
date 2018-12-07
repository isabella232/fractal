use gtk;
use gtk::prelude::*;

pub fn new(parent: &gtk::Window, msg: &str) {
    let dialog = gtk::MessageDialog::new(
        Some(parent),
        gtk::DialogFlags::MODAL,
        gtk::MessageType::Warning,
        gtk::ButtonsType::Ok,
        &msg,
    );
    dialog.connect_response(move |d, _| {
        d.destroy();
    });
    dialog.show();
}
