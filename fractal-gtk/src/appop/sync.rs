use crate::util::i18n::i18n;

use crate::app::{App, RUNTIME};
use crate::appop::AppOp;
use crate::backend::{
    sync::{self, RoomElement, SyncRet},
    HandleError,
};

impl AppOp {
    pub fn initial_sync(&self, show: bool) {
        if show {
            self.inapp_notify(&i18n("Syncing, this could take a while"));
        } else {
            self.hide_inapp_notify();
        }
    }

    pub fn sync(&mut self, initial: bool, number_tries: u32) {
        if let (Some(login_data), false) = (self.login_data.clone(), self.syncing) {
            self.syncing = true;
            // for the initial sync we set the since to None to avoid long syncing
            // the since can be a very old value and following the spec we should
            // do the initial sync without a since:
            // https://matrix.org/docs/spec/client_server/latest.html#syncing
            let join_to_room = self.join_to_room.clone();
            let since = self.since.clone().filter(|_| !initial);
            RUNTIME.spawn(async move {
                let query = sync::sync(
                    login_data.session_client,
                    login_data.uid,
                    join_to_room,
                    since,
                    initial,
                    number_tries,
                )
                .await;

                match query {
                    Ok(SyncRet::NoSince {
                        rooms,
                        default,
                        next_batch,
                    }) => {
                        let clear_room_list = true;
                        APPOP!(set_rooms, (rooms, clear_room_list));
                        // Open the newly joined room
                        let jtr = default.as_ref().map(|r| r.id.clone());
                        APPOP!(set_join_to_room, (jtr));
                        if let Some(room) = default {
                            let room_id = room.id;
                            APPOP!(set_active_room_by_id, (room_id));
                        }

                        let s = Some(next_batch);
                        APPOP!(synced, (s));
                    }
                    Ok(SyncRet::WithSince {
                        update_rooms,
                        room_notifications,
                        update_rooms_2,
                        other,
                        next_batch,
                    }) => {
                        let clear_room_list = false;
                        let msgs: Vec<_> = update_rooms
                            .iter()
                            .flat_map(|r| &r.messages)
                            .cloned()
                            .collect();
                        APPOP!(set_rooms, (update_rooms, clear_room_list));
                        APPOP!(show_room_messages, (msgs));

                        let clear_room_list = false;
                        APPOP!(set_rooms, (update_rooms_2, clear_room_list));

                        for (room_id, unread_notifications) in room_notifications {
                            let r = room_id;
                            let n: u64 = unread_notifications
                                .notification_count
                                .map(Into::into)
                                .unwrap_or_default();
                            let h: u64 = unread_notifications
                                .highlight_count
                                .map(Into::into)
                                .unwrap_or_default();
                            APPOP!(set_room_notifications, (r, n, h));
                        }

                        for room_element in other {
                            match room_element {
                                RoomElement::Name(room_id, name) => {
                                    let n = Some(name);
                                    APPOP!(room_name_change, (room_id, n));
                                }
                                RoomElement::Topic(room_id, topic) => {
                                    let t = Some(topic);
                                    APPOP!(room_topic_change, (room_id, t));
                                }
                                RoomElement::NewAvatar(room_id) => {
                                    APPOP!(new_room_avatar, (room_id));
                                }
                                RoomElement::MemberEvent(event) => {
                                    APPOP!(room_member_event, (event));
                                }
                                RoomElement::RemoveMessage(room_id, msg_id) => {
                                    APPOP!(remove_message, (room_id, msg_id));
                                }
                            }
                        }

                        let s = Some(next_batch);
                        APPOP!(synced, (s));
                    }
                    Err(err) => {
                        err.handle_error();
                    }
                }
            });
        }
    }

    pub fn synced(&mut self, since: Option<String>) {
        self.syncing = false;
        self.since = since;
        self.sync(false, 0);
        self.initial_sync(false);
    }

    pub fn sync_error(&mut self, number_tries: u32) {
        self.syncing = false;
        self.sync(false, number_tries);
    }
}
