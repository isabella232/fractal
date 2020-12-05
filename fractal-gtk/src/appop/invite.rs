use crate::app::RUNTIME;
use crate::appop::member::SearchType;
use crate::appop::AppOp;
use crate::backend::room;
use crate::backend::HandleError;
use crate::model::member::Member;
use matrix_sdk::identifiers::RoomId;

impl AppOp {
    pub fn add_to_invite(&mut self, member: Member) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        let user_info_cache = self.user_info_cache.clone();

        self.ui
            .add_to_invite(session_client, user_info_cache, member, self.search_type);
    }

    pub fn detect_removed_invite(&mut self) {
        self.ui.detect_removed_invite(self.search_type);
    }

    pub fn show_invite_user_dialog(&mut self) {
        self.search_type = SearchType::Invite;
        let room_name = if let Some(ref aroom) = self.active_room {
            self.rooms.get(aroom).and_then(|r| r.name.as_deref())
        } else {
            None
        };

        self.ui.show_invite_user_dialog(room_name);
    }

    pub fn invite(&mut self) {
        let login_data = unwrap_or_unit_return!(self.login_data.clone());
        if let Some(ref r) = self.active_room {
            for user in &self.ui.invite_list {
                let session_client = login_data.session_client.clone();
                let room_id = r.clone();
                let user_id = user.0.uid.clone();
                RUNTIME.spawn(async move {
                    let query = room::invite(session_client, &room_id, &user_id).await;
                    if let Err(err) = query {
                        err.handle_error();
                    }
                });
            }
        }
        self.ui.close_invite_dialog();
    }

    pub fn remove_inv(&mut self, room_id: &RoomId) {
        self.rooms.remove(room_id);
        self.ui.roomlist.remove_room(room_id);
    }

    pub fn accept_inv(&mut self, accept: bool) {
        let room_id = unwrap_or_unit_return!(self.invitation_roomid.take().clone());
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        self.remove_inv(&room_id);
        RUNTIME.spawn(async move {
            if accept {
                match room::join_room(session_client, &room_id.into()).await {
                    Ok(jtr) => {
                        let jtr = Some(jtr);
                        APPOP!(set_join_to_room, (jtr));
                        APPOP!(reload_rooms);
                    }
                    Err(err) => {
                        err.handle_error();
                    }
                }
            } else {
                let query = room::leave_room(session_client, &room_id).await;
                if let Err(err) = query {
                    err.handle_error();
                }
            }
        });
    }

    pub fn set_invite_user_dialog_placeholder(&mut self) {
        self.ui.set_invite_user_dialog_placeholder(self.search_type);
    }

    pub fn remove_invite_user_dialog_placeholder(&mut self) {
        self.ui
            .remove_invite_user_dialog_placeholder(self.search_type);
    }
}
