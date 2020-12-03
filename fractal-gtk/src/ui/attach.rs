use super::UI;
use crate::util::get_pixbuf_data;
use crate::util::i18n::i18n;
use crate::APPOP;
use anyhow::Error;
use gdk_pixbuf::Pixbuf;
use glib::clone;
use gtk::prelude::*;
use std::fs::File;
use std::io::prelude::*;
use std::path::PathBuf;

impl UI {
    fn draw_image_paste_dialog(&self, pixb: &Pixbuf) {
        let w = pixb.get_width();
        let h = pixb.get_height();
        let scaled = if w > 600 {
            pixb.scale_simple(600, h * 600 / w, gdk_pixbuf::InterpType::Bilinear)
        } else {
            Some(pixb.clone())
        };

        if let Some(pb) = scaled {
            let img = gtk::Image::new();
            let dialog = gtk::Dialog::with_buttons(
                Some(i18n("Image from Clipboard").as_str()),
                Some(&self.main_window),
                gtk::DialogFlags::MODAL
                    | gtk::DialogFlags::USE_HEADER_BAR
                    | gtk::DialogFlags::DESTROY_WITH_PARENT,
                &[],
            );

            img.set_from_pixbuf(Some(&pb));
            img.show();
            dialog.get_content_area().add(&img);
            dialog.present();

            if let Some(hbar) = dialog.get_header_bar() {
                let headerbar = hbar.downcast::<gtk::HeaderBar>().unwrap();
                let closebtn = gtk::Button::with_label(i18n("Cancel").as_str());
                let okbtn = gtk::Button::with_label(i18n("Send").as_str());
                okbtn.get_style_context().add_class("suggested-action");

                headerbar.set_show_close_button(false);
                headerbar.pack_start(&closebtn);
                headerbar.pack_end(&okbtn);
                headerbar.show_all();

                closebtn.connect_clicked(clone!(@strong dialog => move |_| {
                    dialog.close();
                }));
                /* FIXME: make this an action */
                okbtn.connect_clicked(clone!(@strong pixb, @strong dialog => move |_| {
                    if let Ok(path) = store_pixbuf(&pixb) {
                        APPOP!(attach_message, (path))
                    }
                    dialog.close();
                }));

                okbtn.grab_focus();
            }
        }
    }

    pub fn paste(&self) {
        if let Some(display) = gdk::Display::get_default() {
            if let Some(clipboard) = gtk::Clipboard::get_default(&display) {
                if clipboard.wait_is_image_available() {
                    if let Some(pixb) = clipboard.wait_for_image() {
                        self.draw_image_paste_dialog(&pixb);

                        // removing text from clipboard
                        clipboard.set_text("");
                        clipboard.set_image(&pixb);
                    }
                } else {
                    // TODO: manage code pasting
                }
            }
        }
    }
}

// TODO: Make async
fn store_pixbuf(pixb: &Pixbuf) -> Result<PathBuf, Error> {
    let data = get_pixbuf_data(pixb)?;
    let mut path = glib::get_tmp_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    /* Filename for the attached image */
    path.push(format!("{}.png", i18n("image")));
    let mut f = File::create(&path)?;
    f.write_all(&data)?;
    f.sync_data()?;

    Ok(path)
}
