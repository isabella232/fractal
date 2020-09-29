use crate::backend::{user, HandleError};
use either::Either;
use glib::clone;
use gtk::prelude::*;
use matrix_sdk::{
    events::{
        room::member::{MemberEventContent, MembershipState},
        StateEvent,
    },
    identifiers::{RoomId, UserId},
};
use url::Url;

use std::collections::HashMap;
use std::convert::TryFrom;

use crate::actions::AppState;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::widgets;
use crate::App;

use crate::model::member::Member;

#[derive(Debug, Clone)]
pub enum SearchType {
    Invite,
    DirectChat,
}

impl AppOp {
    pub fn member_level(&self, member: &Member) -> i64 {
        self.active_room
            .as_ref()
            .and_then(|a_room| self.rooms.get(a_room)?.admins.get(&member.uid))
            .copied()
            .unwrap_or(0)
    }

    pub fn set_room_members(&mut self, room_id: RoomId, members: Vec<Member>) {
        if let Some(r) = self.rooms.get_mut(&room_id) {
            r.members = HashMap::new();
            for m in members {
                r.members.insert(m.uid.clone(), m);
            }
        }

        self.recalculate_room_name(room_id);

        /* FIXME: update the current room settings insteat of creating a new one */
        if self.room_settings.is_some() && self.state == AppState::RoomSettings {
            self.create_room_settings();
        }
    }

    pub fn room_member_event(&mut self, ev: StateEvent<MemberEventContent>) {
        // NOTE: maybe we should show this events in the message list to notify enters and leaves
        // to the user

        let sender = ev.sender;
        match ev.content.membership {
            MembershipState::Leave => {
                if let Some(r) = self.rooms.get_mut(&ev.room_id) {
                    r.members.remove(&sender);
                }
            }
            MembershipState::Join => {
                let m = Member {
                    avatar: ev
                        .content
                        .avatar_url
                        .and_then(|u| Url::parse(&u).ok())
                        .map(Either::Left),
                    alias: ev.content.displayname,
                    uid: sender,
                };
                if let Some(r) = self.rooms.get_mut(&ev.room_id) {
                    r.members.insert(m.uid.clone(), m);
                }
            }
            // ignoring other memberships
            _ => {}
        }
    }

    pub fn user_search_finished(&self, users: Vec<Member>) {
        match self.search_type {
            SearchType::Invite => {
                let entry = self
                    .ui
                    .builder
                    .get_object::<gtk::TextView>("invite_entry")
                    .expect("Can't find invite_entry in ui file.");
                let listbox = self
                    .ui
                    .builder
                    .get_object::<gtk::ListBox>("user_search_box")
                    .expect("Can't find user_search_box in ui file.");
                let scroll = self
                    .ui
                    .builder
                    .get_object::<gtk::Widget>("user_search_scroll")
                    .expect("Can't find user_search_scroll in ui file.");

                if let Some(buffer) = entry.get_buffer() {
                    let start = buffer.get_start_iter();
                    let end = buffer.get_end_iter();

                    self.search_finished(
                        users,
                        listbox,
                        scroll,
                        buffer
                            .get_text(&start, &end, false)
                            .map(|gstr| gstr.to_string()),
                    );
                }
            }
            SearchType::DirectChat => {
                let entry = self
                    .ui
                    .builder
                    .get_object::<gtk::TextView>("to_chat_entry")
                    .expect("Can't find to_chat_entry in ui file.");
                let listbox = self
                    .ui
                    .builder
                    .get_object::<gtk::ListBox>("direct_chat_search_box")
                    .expect("Can't find direct_chat_search_box in ui file.");
                let scroll = self
                    .ui
                    .builder
                    .get_object::<gtk::Widget>("direct_chat_search_scroll")
                    .expect("Can't find direct_chat_search_scroll in ui file.");

                if let Some(buffer) = entry.get_buffer() {
                    let start = buffer.get_start_iter();
                    let end = buffer.get_end_iter();

                    self.search_finished(
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
    }

    pub fn search_finished(
        &self,
        mut users: Vec<Member>,
        listbox: gtk::ListBox,
        scroll: gtk::Widget,
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

        for (i, u) in users.iter().enumerate() {
            let w;
            {
                let mb = widgets::MemberBox::new(u, &self);
                w = mb.widget(true);
            }

            w.connect_button_press_event(clone!(@strong u => move |_, _| {
                /* FIXME: Create Action */
                let u = u.clone();
                APPOP!(add_to_invite, (u));
                glib::signal::Inhibit(true)
            }));

            listbox.insert(&w, i as i32);
            scroll.show();
        }
    }

    pub fn search_invite_user(&self, term: String) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        RUNTIME.spawn(async move {
            match user::search(session_client, &term).await {
                Ok(users) => {
                    APPOP!(user_search_finished, (users));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }
}
