use glib::clone;
use gtk::prelude::*;
use sourceview4::prelude::*;

use crate::util;

use crate::appop::AppOp;

pub fn connect(appop: &AppOp) {
    let app_tx = appop.app_tx.clone();
    let md_popover_btn = &appop.ui.sventry.markdown;
    let md_img = appop.ui.sventry.markdown_img.clone();
    let buffer = appop.ui.sventry.buffer.clone();

    let popover: gtk::Popover = appop
        .ui
        .builder
        .get_object("markdown_popover")
        .expect("Couldn't find markdown_popover in ui file.");

    let markdown_switch: gtk::Switch = appop
        .ui
        .builder
        .get_object("markdown_switch")
        .expect("Couldn't find markdown_switch in ui file.");

    let txt: gtk::Grid = appop
        .ui
        .builder
        .get_object("tutorial_text_box")
        .expect("Couldn't find tutorial_text_box in ui file.");

    let md_lang =
        sourceview4::LanguageManager::get_default().and_then(|lm| lm.get_language("markdown"));

    md_popover_btn.set_popover(Some(&popover));

    let md_active = util::get_markdown_schema();
    if md_active {
        let _ = app_tx.send(Box::new(|op| {
            op.md_enabled = true;
        }));
        markdown_switch.set_active(true);
        md_img.set_from_icon_name(Some("format-indent-more-symbolic"), gtk::IconSize::Menu);
        txt.get_style_context().remove_class("dim-label");

        if let Some(md_lang) = md_lang.clone() {
            buffer.set_highlight_matching_brackets(true);
            buffer.set_language(Some(&md_lang));
            buffer.set_highlight_syntax(true);
        }
    }

    markdown_switch.connect_property_active_notify(clone!(@strong markdown_switch => move |_| {
        let md_active = markdown_switch.get_active();
        let _ = app_tx.send(Box::new(move |op| {
            op.md_enabled = md_active;
        }));

        if markdown_switch.get_active() {
            md_img.set_from_icon_name(
                Some("format-indent-more-symbolic"),
                gtk::IconSize::Menu,
            );
            txt.get_style_context().remove_class("dim-label");
            util::set_markdown_schema(true);

            if let Some(md_lang) = md_lang.clone() {
                buffer.set_highlight_matching_brackets(true);
                buffer.set_language(Some(&md_lang));
                buffer.set_highlight_syntax(true);
            }
        } else {
            md_img.set_from_icon_name(
                Some("format-justify-left-symbolic"),
                gtk::IconSize::Menu,
            );
            txt.get_style_context().add_class("dim-label");
            util::set_markdown_schema(false);

            let lang: Option<&sourceview4::Language> = None;
            buffer.set_highlight_matching_brackets(false);
            buffer.set_language(lang);
            buffer.set_highlight_syntax(false);
        }
    }));
}
