use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::backend::{
    sync::{self, RoomElement, SyncRet, SyncUpdates},
    HandleError,
};
use crate::model::{
    member::Member,
    room::{Room, RoomMembership, RoomTag},
};
use crate::util::i18n::i18n;
use matrix_sdk::deserialized_responses::{JoinedRoom, SyncResponse};
use matrix_sdk::events::AnyEphemeralRoomEventContent;
use matrix_sdk::events::AnySyncMessageEvent;
use matrix_sdk::events::AnySyncRoomEvent;
use matrix_sdk::events::AnySyncStateEvent;
use matrix_sdk::identifiers::{RoomId, UserId};
use std::collections::BTreeMap;

impl AppOp {
    pub fn initial_sync(&self, show: bool) {
        if show {
            self.ui
                .inapp_notify(&i18n("Syncing, this could take a while"));
        } else {
            self.ui.hide_inapp_notify();
        }
    }

    pub fn sync(&mut self, initial: bool, number_tries: u32) {
        if let (Some((session_client, user_id)), false) = (
            self.login_data
                .as_ref()
                .map(|ld| (ld.session_client.clone(), ld.uid.clone())),
            self.syncing,
        ) {
            self.syncing = true;
            // for the initial sync we set the since to None to avoid long syncing
            // the since can be a very old value and following the spec we should
            // do the initial sync without a since:
            // https://matrix.org/docs/spec/client_server/latest.html#syncing
            let join_to_room = self.join_to_room.clone();
            let since = self.since.clone().filter(|_| !initial);
            RUNTIME.spawn(async move {
                let query = sync::sync(session_client, since, number_tries).await;

                match query {
                    Ok(response) => {
                        let sync_ret = transform_sync_response(response, initial, user_id);
                        let clear_room_list = sync_ret.updates.is_none();
                        if let Some(updates) = sync_ret.updates {
                            let rooms = sync_ret.rooms;
                            let msgs: Vec<_> =
                                rooms.iter().flat_map(|r| &r.messages).cloned().collect();
                            APPOP!(set_rooms, (rooms, clear_room_list));
                            APPOP!(show_room_messages, (msgs));
                            let typing_events_as_rooms = updates.typing_events_as_rooms;
                            APPOP!(set_rooms, (typing_events_as_rooms, clear_room_list));

                            for (room_id, unread_notifications) in updates.room_notifications {
                                let r = room_id;
                                let n: u64 = unread_notifications.notification_count;
                                let h: u64 = unread_notifications.highlight_count;
                                APPOP!(set_room_notifications, (r, n, h));
                            }

                            for room_element in updates.new_events {
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
                        } else {
                            let rooms = sync_ret.rooms;
                            let jtr = join_to_room.and_then(|jtr| {
                                rooms
                                    .iter()
                                    .map(|room| &room.id)
                                    .find(|rid| **rid == jtr)
                                    .cloned()
                            });
                            APPOP!(set_rooms, (rooms, clear_room_list));
                            // Open the newly joined room
                            let jtr_ = jtr.clone();
                            APPOP!(set_join_to_room, (jtr_));
                            if let Some(room_id) = jtr {
                                APPOP!(set_active_room_by_id, (room_id));
                            }
                        }

                        let s = Some(sync_ret.next_batch);
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

fn transform_sync_response(response: SyncResponse, initial: bool, user_id: UserId) -> SyncRet {
    let updates = if initial {
        None
    } else {
        Some(get_sync_updates(&response.rooms.join, &user_id))
    };

    SyncRet {
        rooms: Room::from_sync_response(&response, user_id),
        next_batch: response.next_batch,
        updates,
    }
}

fn get_sync_updates(join: &BTreeMap<RoomId, JoinedRoom>, user_id: &UserId) -> SyncUpdates {
    SyncUpdates {
        room_notifications: join
            .iter()
            .map(|(k, room)| (k.clone(), room.unread_notifications.clone()))
            .collect(),
        typing_events_as_rooms: join
            .iter()
            .map(|(k, room)| {
                let typing: Vec<Member> = room.ephemeral.events
                    .iter()
                    .filter_map(|event| match event.content() {
                        AnyEphemeralRoomEventContent::Typing(content) => {
                            Some(content.user_ids)
                        }
                        _ => None,
                    })
                    .flatten()
                    // ignoring the user typing notifications
                    .filter(|user| user != user_id)
                    .map(|uid| Member {
                        uid,
                        alias: None,
                        avatar: None,
                    })
                    .collect();

                Room {
                    typing_users: typing,
                    ..Room::new(k.clone(), RoomMembership::Joined(RoomTag::None))
                }
            })
            .collect(),
        new_events: join
            .iter()
            .flat_map(|(room_id, room)| {
                let room_id = room_id.clone();
                room.timeline
                    .events
                    .iter()
                    .map(move |ev| (room_id.clone(), ev))
            })
            .filter_map(|(room_id, event)| match event {
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomName(ev)) => {
                    let name = ev.content.name().map(Into::into).unwrap_or_default();
                    Some(RoomElement::Name(room_id, name))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomTopic(ev)) => {
                    Some(RoomElement::Topic(room_id, ev.content.topic.clone()))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomAvatar(_)) => {
                    Some(RoomElement::NewAvatar(room_id))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomMember(ev)) => Some(
                    RoomElement::MemberEvent(ev.clone().into_full_event(room_id)),
                ),
                AnySyncRoomEvent::Message(AnySyncMessageEvent::RoomRedaction(ev)) => {
                    Some(RoomElement::RemoveMessage(room_id, ev.redacts.clone()))
                }
                _ => None,
            })
            .collect(),
    }
}
