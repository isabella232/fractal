use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::globals;
use crate::model::{
    member::Member,
    room::{Room, RoomMembership, RoomTag},
};
use crate::util::i18n::i18n;
use log::warn;
use matrix_sdk::api::r0::filter::Filter as EventFilter;
use matrix_sdk::api::r0::filter::FilterDefinition;
use matrix_sdk::api::r0::filter::LazyLoadOptions;
use matrix_sdk::api::r0::filter::RoomEventFilter;
use matrix_sdk::api::r0::filter::RoomFilter;
use matrix_sdk::api::r0::sync::sync_events::Filter;
use matrix_sdk::api::r0::sync::sync_events::JoinedRoom;
use matrix_sdk::api::r0::sync::sync_events::Response as SyncResponse;
use matrix_sdk::api::r0::sync::sync_events::UnreadNotificationsCount;
use matrix_sdk::assign;
use matrix_sdk::events::room::member::MemberEventContent;
use matrix_sdk::events::AnyEphemeralRoomEventContent;
use matrix_sdk::events::AnySyncMessageEvent;
use matrix_sdk::events::AnySyncRoomEvent;
use matrix_sdk::events::AnySyncStateEvent;
use matrix_sdk::events::StateEvent;
use matrix_sdk::identifiers::{EventId, RoomId, UserId};
use matrix_sdk::Client as MatrixClient;
use matrix_sdk::LoopCtrl;
use matrix_sdk::SyncSettings;
use std::{
    collections::{BTreeMap, HashMap},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
};

enum RoomElement {
    Name(RoomId, String),
    Topic(RoomId, String),
    NewAvatar(RoomId),
    MemberEvent(StateEvent<MemberEventContent>),
    RemoveMessage(RoomId, EventId),
}

struct SyncRet {
    // Only new rooms if it's an initial sync
    pub rooms: Vec<Room>,
    pub next_batch: String,
    // None if it's an initial sync
    pub updates: Option<SyncUpdates>,
}

struct SyncUpdates {
    pub room_notifications: HashMap<RoomId, UnreadNotificationsCount>,
    // TODO: Typing events should become RoomElements
    pub typing_events_as_rooms: Vec<Room>,
    pub new_events: Vec<RoomElement>,
}

impl AppOp {
    pub fn show_initial_sync(&self) {
        self.ui
            .inapp_notify(&i18n("Syncing, this could take a while"));
    }

    pub fn hide_initial_sync(&self) {
        self.ui.hide_inapp_notify();
    }

    pub fn setup_sync(&mut self) {
        let (session_client, user_id) = unwrap_or_unit_return!(self
            .login_data
            .as_ref()
            .map(|ld| (ld.session_client.clone(), ld.uid.clone())));

        RUNTIME.spawn(create_and_launch_sync_task(
            session_client,
            user_id,
            self.join_to_room.clone(),
        ));
    }

    pub fn synced(&mut self, since: Option<String>) {
        self.since = since;
        self.hide_initial_sync();
    }
}

async fn create_and_launch_sync_task(
    session_client: MatrixClient,
    user_id: UserId,
    join_to_room: Arc<Mutex<Option<RoomId>>>,
) {
    let timeline_not_types = [String::from("m.call.*")];
    let timeline_types = [String::from("m.room.message"), String::from("m.sticker")];
    let state_types = [String::from("m.room.*")];
    // Don't filter event fields, it breaks deserialization.
    // Clearly the Matrix API is very static-typing-unfriendly right now.
    let filter = assign!(FilterDefinition::empty(), {
        presence: assign!(EventFilter::empty(), {
            types: Some(&[]),
        }),
        room: assign!(RoomFilter::empty(), {
            timeline: assign!(RoomEventFilter::empty(), {
                not_types: &timeline_not_types,
                limit: Some(globals::PAGE_LIMIT.into()),
                types: Some(&timeline_types),
            }),
            ephemeral: assign!(RoomEventFilter::empty(), {
                types: Some(&[]),
            }),
            state: assign!(RoomEventFilter::empty(), {
                types: Some(&state_types),
                lazy_load_options: LazyLoadOptions::Enabled {
                    include_redundant_members: false,
                },
            }),
        }),
    });

    let settings = SyncSettings::new().filter(Filter::FilterDefinition(filter));

    let initial = AtomicBool::from(true);
    let sync_callback = move |response: SyncResponse| {
        let sync_ret =
            transform_sync_response(response, initial.load(Ordering::Relaxed), user_id.clone());
        initial.store(false, Ordering::Relaxed);
        let clear_room_list = sync_ret.updates.is_none();
        if let Some(updates) = sync_ret.updates {
            let rooms = sync_ret.rooms;
            let msgs: Vec<_> = rooms.iter().flat_map(|r| &r.messages).cloned().collect();
            APPOP!(set_rooms, (rooms, clear_room_list));
            APPOP!(show_room_messages, (msgs));
            let typing_events_as_rooms = updates.typing_events_as_rooms;
            APPOP!(set_rooms, (typing_events_as_rooms, clear_room_list));

            for (room_id, unread_notifications) in updates.room_notifications {
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
            let jtr = join_to_room.lock().unwrap().as_ref().and_then(|jtr| {
                rooms
                    .iter()
                    .map(|room| &room.id)
                    .find(|rid| *rid == jtr)
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

        async { LoopCtrl::Continue }
    };

    session_client
        .sync_with_callback(settings, sync_callback)
        .await;
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
                    .map(|ev| ev.deserialize())
                    .inspect(|result_ev| if let Err(err) = result_ev {
                        warn!("Bad event: {}", err);
                    })
                    .filter_map(Result::ok)
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
                    .map(move |ev| Ok((room_id.clone(), ev.deserialize()?)))
            })
            .inspect(|result_ev: &Result<_, serde_json::Error>| {
                if let Err(err) = result_ev {
                    warn!("Bad event: {}", err);
                }
            })
            .filter_map(Result::ok)
            .filter_map(|(room_id, event)| match event {
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomName(ev)) => {
                    let name = ev.content.name().map(Into::into).unwrap_or_default();
                    Some(RoomElement::Name(room_id, name))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomTopic(ev)) => {
                    Some(RoomElement::Topic(room_id, ev.content.topic))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomAvatar(_)) => {
                    Some(RoomElement::NewAvatar(room_id))
                }
                AnySyncRoomEvent::State(AnySyncStateEvent::RoomMember(ev)) => {
                    Some(RoomElement::MemberEvent(ev.into_full_event(room_id)))
                }
                AnySyncRoomEvent::Message(AnySyncMessageEvent::RoomRedaction(ev)) => {
                    Some(RoomElement::RemoveMessage(room_id, ev.redacts))
                }
                _ => None,
            })
            .collect(),
    }
}
