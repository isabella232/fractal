use crate::actions::AppState;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::backend::{user, HandleError};
use crate::model::member::Member;
use crate::model::room::RoomList;
use either::Either;
use matrix_sdk::{
    events::{
        room::member::{MemberEventContent, MembershipState},
        StateEvent,
    },
    identifiers::{RoomId, UserId},
};
use url::Url;

#[derive(Debug, Clone, Copy)]
pub enum SearchType {
    Invite,
    DirectChat,
}

impl AppOp {
    pub fn set_room_members(&mut self, room_id: RoomId, members: Vec<Member>) {
        if let Some(r) = self.rooms.get_mut(&room_id) {
            r.members = members.into_iter().map(|m| (m.uid.clone(), m)).collect();
        }

        self.recalculate_room_name(room_id);

        /* FIXME: update the current room settings insteat of creating a new one */
        if self.ui.room_settings.is_some() && self.state == AppState::RoomSettings {
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
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));

        self.ui.user_search_finished(
            session_client,
            self.user_info_cache.clone(),
            self.active_room.as_ref(),
            &self.rooms,
            users,
            self.search_type,
        );
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

pub fn member_level(active_room: Option<&RoomId>, rooms: &RoomList, member_uid: &UserId) -> i64 {
    active_room
        .and_then(|a_room| rooms.get(a_room)?.admins.get(member_uid))
        .copied()
        .unwrap_or(0)
}
