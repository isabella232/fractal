use crate::globals;
use crate::model::{
    member::Member,
    room::{Room, RoomMembership, RoomTag},
};
use fractal_api::api::r0::filter::Filter as EventFilter;
use fractal_api::api::r0::filter::FilterDefinition;
use fractal_api::api::r0::filter::LazyLoadOptions;
use fractal_api::api::r0::filter::RoomEventFilter;
use fractal_api::api::r0::filter::RoomFilter;
use fractal_api::api::r0::sync::sync_events::Filter;
use fractal_api::api::r0::sync::sync_events::UnreadNotificationsCount;
use fractal_api::assign;
use fractal_api::events::room::member::MemberEventContent;
use fractal_api::events::AnyEphemeralRoomEventContent;
use fractal_api::events::AnySyncMessageEvent;
use fractal_api::events::AnySyncRoomEvent;
use fractal_api::events::AnySyncStateEvent;
use fractal_api::events::StateEvent;
use fractal_api::SyncSettings;

use fractal_api::identifiers::{EventId, RoomId, UserId};
use fractal_api::Client as MatrixClient;
use fractal_api::Error as MatrixError;
use log::error;
use std::{collections::HashMap, time::Duration};

use super::{get_ruma_client_error, remove_matrix_access_token_if_present, HandleError};
use crate::app::App;
use crate::APPOP;

pub enum RoomElement {
    Name(RoomId, String),
    Topic(RoomId, String),
    NewAvatar(RoomId),
    MemberEvent(StateEvent<MemberEventContent>),
    RemoveMessage(RoomId, EventId),
}

#[derive(Debug)]
pub struct SyncError(MatrixError, u32);

impl HandleError for SyncError {
    fn handle_error(&self) {
        let err_str = format!("{:?}", self.0);
        error!(
            "SYNC Error: {}",
            remove_matrix_access_token_if_present(&err_str).unwrap_or(err_str)
        );
        let new_number_tries = self.1 + 1;
        APPOP!(sync_error, (new_number_tries));
    }
}

#[derive(Debug)]
pub struct RoomsError(MatrixError);

impl<T: Into<MatrixError>> From<T> for RoomsError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

impl HandleError for RoomsError {}

#[derive(Debug)]
pub struct UpdateRoomsError(MatrixError);

impl<T: Into<MatrixError>> From<T> for UpdateRoomsError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

impl HandleError for UpdateRoomsError {}

#[derive(Debug)]
pub struct RoomElementError(MatrixError);

impl<T: Into<MatrixError>> From<T> for RoomElementError {
    fn from(err: T) -> Self {
        Self(err.into())
    }
}

impl HandleError for RoomElementError {}

pub enum SyncRet {
    NoSince {
        rooms: Result<(Vec<Room>, Option<Room>), RoomsError>,
        next_batch: String,
    },
    WithSince {
        update_rooms: Result<Vec<Room>, UpdateRoomsError>,
        room_notifications: HashMap<RoomId, UnreadNotificationsCount>,
        update_rooms_2: Result<Vec<Room>, UpdateRoomsError>,
        other: Result<Vec<RoomElement>, RoomElementError>,
        next_batch: String,
    },
}

pub async fn sync(
    session_client: MatrixClient,
    user_id: UserId,
    join_to_room: Option<RoomId>,
    since: Option<String>,
    initial: bool,
    number_tries: u32,
) -> Result<SyncRet, SyncError> {
    let timeline_not_types = [String::from("m.call.*")];
    let timeline_types = [String::from("m.room.message"), String::from("m.sticker")];
    let state_types = [String::from("m.room.*")];
    let sync_settings = if !initial {
        SyncSettings::new().timeout(Duration::from_secs(30))
    } else {
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

        SyncSettings::new().filter(Filter::FilterDefinition(filter))
    };

    let sync_settings = match since.clone() {
        Some(sync_token) => sync_settings.token(sync_token),
        None => sync_settings,
    };

    match session_client.sync_once(sync_settings).await {
        Ok(response) => {
            if since.is_none() {
                let rooms = Room::from_sync_response(&response, user_id)
                    .map(|rooms| {
                        let def = join_to_room
                            .and_then(|jtr| rooms.iter().find(|x| x.id == jtr).cloned());
                        (rooms, def)
                    })
                    .map_err(Into::into);

                let next_batch = response.next_batch;

                Ok(SyncRet::NoSince { rooms, next_batch })
            } else {
                let join = &response.rooms.join;

                // New rooms
                let update_rooms =
                    Room::from_sync_response(&response, user_id.clone()).map_err(Into::into);

                // Room notifications
                let room_notifications = join
                    .iter()
                    .map(|(k, room)| (k.clone(), room.unread_notifications.clone()))
                    .collect();

                // Typing notifications
                let update_rooms_2 = join
                    .iter()
                    .map(|(k, room)| {
                        let typing: Vec<Member> = room.ephemeral.events
                            .iter()
                            .map(|ev| ev.deserialize())
                            .collect::<Result<Vec<_>, _>>()?
                            .into_iter()
                            .filter_map(|event| match event.content() {
                                AnyEphemeralRoomEventContent::Typing(content) => {
                                    Some(content.user_ids)
                                }
                                _ => None,
                            })
                            .flatten()
                            // ignoring the user typing notifications
                            .filter(|user| *user != user_id)
                            .map(|uid| Member {
                                uid,
                                alias: None,
                                avatar: None,
                            })
                            .collect();

                        Ok(Room {
                            typing_users: typing,
                            ..Room::new(k.clone(), RoomMembership::Joined(RoomTag::None))
                        })
                    })
                    .collect();

                // Other events
                let other = join
                    .iter()
                    .flat_map(|(room_id, room)| {
                        let room_id = room_id.clone();
                        room.timeline
                            .events
                            .iter()
                            .map(move |ev| Ok((room_id.clone(), ev.deserialize()?)))
                    })
                    .filter_map(|rid_ev| {
                        let (room_id, event) = match rid_ev {
                            Ok(roomid_event) => roomid_event,
                            Err(err) => return Some(Err(err)),
                        };

                        match event {
                            AnySyncRoomEvent::State(AnySyncStateEvent::RoomName(ev)) => {
                                let name = ev.content.name().map(Into::into).unwrap_or_default();
                                Some(Ok(RoomElement::Name(room_id, name)))
                            }
                            AnySyncRoomEvent::State(AnySyncStateEvent::RoomTopic(ev)) => {
                                Some(Ok(RoomElement::Topic(room_id, ev.content.topic)))
                            }
                            AnySyncRoomEvent::State(AnySyncStateEvent::RoomAvatar(_)) => {
                                Some(Ok(RoomElement::NewAvatar(room_id)))
                            }
                            AnySyncRoomEvent::State(AnySyncStateEvent::RoomMember(ev)) => {
                                Some(Ok(RoomElement::MemberEvent(ev.into_full_event(room_id))))
                            }
                            AnySyncRoomEvent::Message(AnySyncMessageEvent::RoomRedaction(ev)) => {
                                Some(Ok(RoomElement::RemoveMessage(room_id, ev.redacts)))
                            }
                            AnySyncRoomEvent::Message(AnySyncMessageEvent::RoomMessage(_)) => None,
                            AnySyncRoomEvent::Message(AnySyncMessageEvent::Sticker(_)) => {
                                // This event is managed in the room list
                                None
                            }
                            ev => {
                                error!("EVENT NOT MANAGED: {:?}", ev);
                                None
                            }
                        }
                    })
                    .collect();

                let next_batch = response.next_batch;

                Ok(SyncRet::WithSince {
                    update_rooms,
                    room_notifications,
                    update_rooms_2,
                    other,
                    next_batch,
                })
            }
        }
        Err(err) => {
            // we wait if there's an error to avoid 100% CPU
            // we wait even longer, if it's a 429 (Too Many Requests) error
            let waiting_time = Duration::from_secs(match get_ruma_client_error(&err) {
                Some(ruma_err) if ruma_err.status_code.as_u16() == 429 => {
                    10 * 2_u64.pow(number_tries)
                }
                _ => 10,
            });
            error!(
                "Sync Error, waiting {} seconds to respond for the next sync",
                waiting_time.as_secs()
            );
            tokio::time::delay_for(waiting_time).await;

            Err(SyncError(err, number_tries))
        }
    }
}
