use super::UI;
use crate::appop::member::member_level;
use crate::appop::member::SearchType;
use crate::appop::UserInfoCache;
use crate::cache::download_to_cache;
use crate::globals;
use crate::model::member::Member;
use crate::model::room::RoomList;
use crate::widgets;
use crate::widgets::AvatarExt;
use crate::APPOP;
use gtk::prelude::*;
use matrix_sdk::identifiers::{RoomId, UserId};
use matrix_sdk::Client as MatrixClient;
use std::convert::TryFrom;

impl UI {
    pub fn user_search_finished(
        &self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        active_room: Option<&RoomId>,
        rooms: &RoomList,
        users: Vec<Member>,
        search_type: SearchType,
    ) {
        let (entry, listbox, scroll) = match search_type {
            SearchType::Invite => {
                let entry = self
                    .builder
                    .get_object::<gtk::TextView>("invite_entry")
                    .expect("Can't find invite_entry in ui file.");
                let listbox = self
                    .builder
                    .get_object::<gtk::ListBox>("user_search_box")
                    .expect("Can't find user_search_box in ui file.");
                let scroll = self
                    .builder
                    .get_object::<gtk::ScrolledWindow>("user_search_scroll")
                    .expect("Can't find user_search_scroll in ui file.");

                (entry, listbox, scroll)
            }
            SearchType::DirectChat => (
                self.direct_chat_dialog.to_chat_entry.clone(),
                self.direct_chat_dialog.search_box.clone(),
                self.direct_chat_dialog.search_scroll.clone(),
            ),
        };

        if let Some(buffer) = entry.get_buffer() {
            let start = buffer.get_start_iter();
            let end = buffer.get_end_iter();

            search_finished(
                session_client,
                user_info_cache,
                active_room,
                rooms,
                users,
                listbox,
                scroll,
                buffer
                    .get_text(&start, &end, false)
                    .map(|gstr| gstr.to_string()),
            );
        }
    }
}

fn search_finished(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    active_room: Option<&RoomId>,
    rooms: &RoomList,
    mut users: Vec<Member>,
    listbox: gtk::ListBox,
    scroll: gtk::ScrolledWindow,
    term: Option<String>,
) {
    for ch in listbox.get_children().iter() {
        listbox.remove(ch);
    }
    scroll.hide();

    let uid_term = term.and_then(|t| UserId::try_from(t.as_str()).ok());
    // Adding a new user if the user
    if let Some(uid) = uid_term {
        if users.iter().find(|u| u.uid == uid).is_none() {
            let member = Member {
                avatar: None,
                alias: None,
                uid,
            };
            users.insert(0, member);
        }
    }

    for (i, member) in users.into_iter().enumerate() {
        let member_level = member_level(active_room, rooms, &member.uid);
        let w = build_memberbox_widget(
            session_client.clone(),
            user_info_cache.clone(),
            member.clone(),
            member_level,
            true,
        );

        w.connect_button_press_event(move |_, _| {
            /* FIXME: Create Action */
            let member = member.clone();
            APPOP!(add_to_invite, (member));
            glib::signal::Inhibit(true)
        });

        listbox.insert(&w, i as i32);
        scroll.show();
    }
}

pub fn build_memberbox_widget(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    member: Member,
    member_level: i64,
    show_uid: bool,
) -> gtk::EventBox {
    let username = gtk::Label::new(None);
    let uid = gtk::Label::new(None);
    let event_box = gtk::EventBox::new();
    let w = gtk::Box::new(gtk::Orientation::Horizontal, 5);
    let v = gtk::Box::new(gtk::Orientation::Vertical, 0);

    uid.set_text(&member.uid.to_string());
    uid.set_valign(gtk::Align::Start);
    uid.set_halign(gtk::Align::Start);
    uid.get_style_context().add_class("member-uid");

    username.set_text(&member.get_alias());
    let mut alias = member.get_alias();
    alias.push_str("\n");
    alias.push_str(&member.uid.to_string());
    username.set_tooltip_text(Some(&alias[..]));
    username.set_margin_end(5);
    username.set_ellipsize(pango::EllipsizeMode::End);
    username.set_valign(gtk::Align::Center);
    username.set_halign(gtk::Align::Start);
    username.get_style_context().add_class("member");

    let avatar = widgets::Avatar::avatar_new(Some(globals::USERLIST_ICON_SIZE));
    let badge = match member_level {
        0..=49 => None,
        50..=99 => Some(widgets::AvatarBadgeColor::Silver),
        _ => Some(widgets::AvatarBadgeColor::Gold),
    };
    let data = avatar.circle(
        member.uid.to_string(),
        Some(alias),
        globals::USERLIST_ICON_SIZE,
        badge,
        None,
    );

    download_to_cache(session_client, user_info_cache, member.uid, data);

    avatar.set_margin_start(3);
    avatar.set_valign(gtk::Align::Center);

    v.set_margin_start(3);
    v.pack_start(&username, true, true, 0);
    if show_uid {
        v.pack_start(&uid, true, true, 0);
    }

    w.add(&avatar);
    w.add(&v);

    event_box.add(&w);
    event_box.show_all();
    event_box
}
