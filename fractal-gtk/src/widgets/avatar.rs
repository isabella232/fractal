use std::boxed::Box;
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use crate::util::cache_dir_path;
use gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use libhandy::AvatarExt as HdyAvatarExt;

pub enum AvatarBadgeColor {
    Gold,
    Silver,
    Grey,
}

pub type Avatar = gtk::Overlay;

pub struct AvatarData {
    id: String,
    pub widget: libhandy::Avatar,
}

impl AvatarData {
    pub fn redraw(&mut self, username: Option<String>) {
        let id = self.id.clone();
        if let Some(n) = username {
            self.widget.set_text(Some(&n));
        }
        // Ensure that we reload the avatar
        self.widget.set_image_load_func(Some(Box::new(move |sz| {
            let path = cache_dir_path(None, &id).unwrap_or_default();
            load_pixbuf(&path, sz)
        })));
    }
}

pub trait AvatarExt {
    fn avatar_new(size: Option<i32>) -> gtk::Overlay;
    fn clean(&self);
    fn create_avatar(&self, size: Option<i32>) -> libhandy::Avatar;
    fn circle(
        &self,
        id: String,
        username: Option<String>,
        size: i32,
        badge: Option<AvatarBadgeColor>,
        badge_size: Option<i32>,
    ) -> Rc<RefCell<AvatarData>>;
}

impl AvatarExt for gtk::Overlay {
    fn clean(&self) {
        for ch in self.get_children().iter() {
            self.remove(ch);
        }
    }

    fn create_avatar(&self, size: Option<i32>) -> libhandy::Avatar {
        let s = size.unwrap_or(40);
        let avatar = libhandy::Avatar::new(s, None, true);
        avatar.set_show_initials(true);
        self.add(&avatar);
        self.show_all();

        avatar
    }

    fn avatar_new(size: Option<i32>) -> gtk::Overlay {
        let b = gtk::Overlay::new();
        b.create_avatar(size);
        b.show_all();
        b.get_style_context().add_class("avatar");

        b
    }
    /// # Arguments
    /// * `id` - User or Room ID
    /// * `username` - Full name
    /// * `size` - Size of the avatar
    /// * `badge_color` - Badge color. None for no badge
    /// * `badge_size` - Badge size. None for size / 3
    fn circle(
        &self,
        id: String,
        username: Option<String>,
        size: i32,
        badge_color: Option<AvatarBadgeColor>,
        badge_size: Option<i32>,
    ) -> Rc<RefCell<AvatarData>> {
        self.clean();
        let avatar = self.create_avatar(Some(size));
        /* remove IRC postfix from the username */
        let username = if let Some(u) = username {
            u.trim_end_matches(" (IRC)")
                .trim_start_matches("#")
                .to_owned()
        } else {
            id.clone()
        };

        avatar.set_text(Some(&username));

        // Power level badge setup
        let badge_size = badge_size.unwrap_or(size / 3);
        if let Some(color) = badge_color {
            let badge = gtk::Box::new(gtk::Orientation::Vertical, 0);
            badge.set_size_request(badge_size, badge_size);
            badge.set_valign(gtk::Align::Start);
            badge.set_halign(gtk::Align::End);
            badge.get_style_context().add_class("badge-circle");
            badge.get_style_context().add_class(match color {
                AvatarBadgeColor::Gold => "badge-gold",
                AvatarBadgeColor::Silver => "badge-silver",
                AvatarBadgeColor::Grey => "badge-grey",
            });
            self.add_overlay(&badge);
        }

        let data = AvatarData {
            id: id.clone(),
            widget: avatar.clone(),
        };
        let avatar_cache: Rc<RefCell<AvatarData>> = Rc::new(RefCell::new(data));

        avatar.set_image_load_func(Some(Box::new(move |sz| {
            let path = cache_dir_path(None, &id).unwrap_or_default();
            load_pixbuf(&path, sz)
        })));

        avatar_cache
    }
}

fn load_pixbuf(path: &Path, size: i32) -> Option<Pixbuf> {
    if let Ok(pixbuf) = Pixbuf::from_file(&path) {
        // FIXME: We end up loading the file twice but we need to load the file first to find out its dimensions to be
        // able to decide wether to scale by width or height and gdk doesn't provide simple API to scale a loaded
        // pixbuf while preserving aspect ratio.
        if pixbuf.get_width() > pixbuf.get_height() {
            Pixbuf::from_file_at_scale(&path, -1, size, true).ok()
        } else {
            Pixbuf::from_file_at_scale(&path, size, -1, true).ok()
        }
    } else {
        None
    }
}
