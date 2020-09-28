use chrono::prelude::*;
use chrono::DateTime;
use fractal_api::{
    events::{
        room::message::{MessageEventContent, RedactedMessageEventContent, Relation},
        sticker::{RedactedStickerEventContent, StickerEventContent},
        AnyMessageEvent, AnyRedactedMessageEvent, AnyRedactedSyncMessageEvent, AnyRoomEvent,
        AnySyncMessageEvent, AnySyncRoomEvent, EventContent, MessageEvent, RedactedMessageEvent,
    },
    identifiers::{EventId, RoomId, UserId},
    url::Url,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::convert::TryFrom;
use std::path::PathBuf;

//FIXME make properties private
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub sender: UserId,
    pub mtype: String,
    pub body: String,
    pub date: DateTime<Local>,
    pub room: RoomId,
    pub thumb: Option<Url>,
    pub local_path_thumb: Option<PathBuf>,
    pub url: Option<Url>,
    pub local_path: Option<PathBuf>,
    // FIXME: This should be a required field but it is mandatory
    // to do it this way because because this struct is used both
    // for received messages and messages to send. At the moment
    // of writing this, using two separate data structures for each
    // use case is just too difficult.
    pub id: Option<EventId>,
    pub formatted_body: Option<String>,
    pub format: Option<String>,
    pub source: Option<String>,
    pub receipt: HashMap<UserId, i64>, // This `HashMap` associates the user ID with a timestamp
    pub redacted: bool,
    // The event ID of the message this is in reply to.
    pub in_reply_to: Option<EventId>,
    // The event ID of the message this replaces.
    pub replace: Option<EventId>,
    // This can be used for the client to add more values to the message on sending
    // for example for images attachment the "info" field can be attached as
    // Some(json!({"info": {"h": 296, "w": 296, "mimetype": "image/png", "orientation": 0, "size": 8796}});
    pub extra_content: Option<JsonValue>,
}

impl PartialEq for Message {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for Message {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self == other {
            Some(Ordering::Equal)
        } else {
            self.date.partial_cmp(&other.date)
        }
    }
}

impl From<MessageEvent<MessageEventContent>> for Message {
    fn from(msg: MessageEvent<MessageEventContent>) -> Self {
        let source = serde_json::to_string_pretty(&msg).ok();

        let initial_message = Self {
            sender: msg.sender,
            date: msg.origin_server_ts.into(),
            room: msg.room_id,
            // It is mandatory for a message event to have
            // an event_id field
            id: Some(msg.event_id),
            mtype: String::new(),
            body: String::new(),
            url: None,
            local_path: None,
            thumb: None,
            local_path_thumb: None,
            formatted_body: None,
            format: None,
            source,
            receipt: HashMap::new(),
            redacted: false,
            in_reply_to: None,
            replace: None,
            extra_content: None,
        };

        match msg.content {
            MessageEventContent::Audio(content) => Self {
                mtype: String::from("m.audio"),
                body: content.body,
                url: content.url.and_then(|u| Url::parse(&u).ok()),
                ..initial_message
            },
            MessageEventContent::File(content) => {
                let url = content.url.and_then(|u| Url::parse(&u).ok());
                Self {
                    mtype: String::from("m.file"),
                    body: content.body,
                    url: url.clone(),
                    thumb: content
                        .info
                        .and_then(|c_info| Url::parse(&c_info.thumbnail_url?).ok())
                        .or(url),
                    ..initial_message
                }
            }
            MessageEventContent::Image(content) => {
                let url = content.url.and_then(|u| Url::parse(&u).ok());
                Self {
                    mtype: String::from("m.image"),
                    body: content.body,
                    url: url.clone(),
                    thumb: content
                        .info
                        .and_then(|c_info| Url::parse(&c_info.thumbnail_url?).ok())
                        .or(url),
                    ..initial_message
                }
            }
            MessageEventContent::Video(content) => {
                let url = content.url.and_then(|u| Url::parse(&u).ok());
                Self {
                    mtype: String::from("m.video"),
                    body: content.body,
                    url: url.clone(),
                    thumb: content
                        .info
                        .and_then(|c_info| Url::parse(&c_info.thumbnail_url?).ok())
                        .or(url),
                    ..initial_message
                }
            }
            MessageEventContent::Text(content) => {
                let (in_reply_to, replace) =
                    content.relates_to.map_or(Default::default(), |r| match r {
                        Relation::Replacement(rep) => (None, Some(rep.event_id)),
                        Relation::Reply { in_reply_to } => (Some(in_reply_to.event_id), None),
                        _ => (None, None),
                    });
                let (body, formatted, in_reply_to) = content.new_content.map_or(
                    (content.body, content.formatted, in_reply_to),
                    |nc| {
                        let in_reply_to = nc.relates_to.and_then(|r| match r {
                            Relation::Reply { in_reply_to } => Some(in_reply_to.event_id),
                            _ => None,
                        });

                        (nc.body, nc.formatted, in_reply_to)
                    },
                );
                let (formatted_body, format) = formatted.map_or(Default::default(), |f| {
                    (Some(f.body), Some(f.format.as_str().into()))
                });

                Self {
                    mtype: String::from("m.text"),
                    body,
                    formatted_body,
                    format,
                    in_reply_to,
                    replace,
                    ..initial_message
                }
            }
            MessageEventContent::Emote(content) => {
                let (formatted_body, format): (Option<String>, Option<String>) =
                    content.formatted.map_or((None, None), |f| {
                        (Some(f.body), Some(f.format.as_str().into()))
                    });
                Self {
                    mtype: String::from("m.emote"),
                    body: content.body,
                    formatted_body,
                    format,
                    ..initial_message
                }
            }
            MessageEventContent::Location(content) => Self {
                mtype: String::from("m.location"),
                body: content.body,
                ..initial_message
            },
            MessageEventContent::Notice(content) => {
                let (in_reply_to, replace) =
                    content.relates_to.map_or(Default::default(), |r| match r {
                        Relation::Replacement(rep) => (None, Some(rep.event_id)),
                        Relation::Reply { in_reply_to } => (Some(in_reply_to.event_id), None),
                        _ => (None, None),
                    });
                let (body, formatted, in_reply_to) = content.new_content.map_or(
                    (content.body, content.formatted, in_reply_to),
                    |nc| {
                        let in_reply_to = nc.relates_to.and_then(|r| match r {
                            Relation::Reply { in_reply_to } => Some(in_reply_to.event_id),
                            _ => None,
                        });

                        (nc.body, nc.formatted, in_reply_to)
                    },
                );
                let (formatted_body, format) = formatted.map_or(Default::default(), |f| {
                    (Some(f.body), Some(f.format.as_str().into()))
                });

                Self {
                    mtype: String::from("m.notice"),
                    body,
                    formatted_body,
                    format,
                    in_reply_to,
                    replace,
                    ..initial_message
                }
            }
            MessageEventContent::ServerNotice(content) => Self {
                mtype: String::from("m.server_notice"),
                body: content.body,
                ..initial_message
            },
            _ => initial_message,
        }
    }
}

impl From<RedactedMessageEvent<RedactedMessageEventContent>> for Message {
    fn from(msg: RedactedMessageEvent<RedactedMessageEventContent>) -> Self {
        let source = serde_json::to_string_pretty(&msg).ok();

        Self {
            sender: msg.sender,
            date: msg.origin_server_ts.into(),
            room: msg.room_id,
            // It is mandatory for a message event to have
            // an event_id field
            id: Some(msg.event_id),
            mtype: String::from(msg.content.event_type()),
            body: String::new(),
            url: None,
            local_path: None,
            thumb: None,
            local_path_thumb: None,
            formatted_body: None,
            format: None,
            source,
            receipt: HashMap::new(),
            redacted: true,
            in_reply_to: None,
            replace: None,
            extra_content: None,
        }
    }
}

impl From<MessageEvent<StickerEventContent>> for Message {
    fn from(msg: MessageEvent<StickerEventContent>) -> Self {
        let source = serde_json::to_string_pretty(&msg).ok();
        let url = Url::parse(&msg.content.url).ok();

        Self {
            sender: msg.sender,
            date: msg.origin_server_ts.into(),
            room: msg.room_id,
            // It is mandatory for a message event to have
            // an event_id field
            id: Some(msg.event_id),
            mtype: String::from(msg.content.event_type()),
            body: msg.content.body,
            url: url.clone(),
            local_path: None,
            thumb: msg
                .content
                .info
                .thumbnail_url
                .and_then(|thumb| Url::parse(&thumb).ok())
                .or(url),
            local_path_thumb: None,
            formatted_body: None,
            format: None,
            source,
            receipt: HashMap::new(),
            redacted: false,
            in_reply_to: None,
            replace: None,
            extra_content: None,
        }
    }
}

impl From<RedactedMessageEvent<RedactedStickerEventContent>> for Message {
    fn from(msg: RedactedMessageEvent<RedactedStickerEventContent>) -> Self {
        let source = serde_json::to_string_pretty(&msg).ok();

        Self {
            sender: msg.sender,
            date: msg.origin_server_ts.into(),
            room: msg.room_id,
            // It is mandatory for a message event to have
            // an event_id field
            id: Some(msg.event_id),
            mtype: String::from(msg.content.event_type()),
            body: String::new(),
            url: None,
            local_path: None,
            thumb: None,
            local_path_thumb: None,
            formatted_body: None,
            format: None,
            source,
            receipt: HashMap::new(),
            redacted: true,
            in_reply_to: None,
            replace: None,
            extra_content: None,
        }
    }
}

impl TryFrom<AnyRoomEvent> for Message {
    type Error = ();

    fn try_from(event: AnyRoomEvent) -> Result<Self, Self::Error> {
        match event {
            AnyRoomEvent::Message(AnyMessageEvent::RoomMessage(room_messages_event)) => {
                Ok(Self::from(room_messages_event))
            }
            AnyRoomEvent::Message(AnyMessageEvent::Sticker(sticker_event)) => {
                Ok(Self::from(sticker_event))
            }
            AnyRoomEvent::RedactedMessage(AnyRedactedMessageEvent::RoomMessage(
                redacted_room_messages_event,
            )) => Ok(Self::from(redacted_room_messages_event)),
            AnyRoomEvent::RedactedMessage(AnyRedactedMessageEvent::Sticker(
                redacted_sticker_event,
            )) => Ok(Self::from(redacted_sticker_event)),
            _ => Err(()),
        }
    }
}

impl TryFrom<(RoomId, AnySyncRoomEvent)> for Message {
    type Error = ();

    fn try_from((room_id, event): (RoomId, AnySyncRoomEvent)) -> Result<Self, Self::Error> {
        match event {
            AnySyncRoomEvent::Message(AnySyncMessageEvent::RoomMessage(room_messages_event)) => {
                Ok(Self::from(room_messages_event.into_full_event(room_id)))
            }
            AnySyncRoomEvent::Message(AnySyncMessageEvent::Sticker(sticker_event)) => {
                Ok(Self::from(sticker_event.into_full_event(room_id)))
            }
            AnySyncRoomEvent::RedactedMessage(AnyRedactedSyncMessageEvent::RoomMessage(
                redacted_room_messages_event,
            )) => Ok(Self::from(
                redacted_room_messages_event.into_full_event(room_id),
            )),
            AnySyncRoomEvent::RedactedMessage(AnyRedactedSyncMessageEvent::Sticker(
                redacted_sticker_event,
            )) => Ok(Self::from(redacted_sticker_event.into_full_event(room_id))),
            _ => Err(()),
        }
    }
}

impl Message {
    pub fn new(
        room: RoomId,
        sender: UserId,
        body: String,
        mtype: String,
        id: Option<EventId>,
    ) -> Self {
        let date = Local::now();
        Message {
            id,
            sender,
            mtype,
            body,
            date,
            room,
            thumb: None,
            local_path_thumb: None,
            url: None,
            local_path: None,
            formatted_body: None,
            format: None,
            source: None,
            receipt: HashMap::new(),
            redacted: false,
            in_reply_to: None,
            replace: None,
            extra_content: None,
        }
    }

    /// Generates an unique transaction id for this message
    /// The txn_id is generated using the md5sum of a concatenation of the message room id, the
    /// message body and the date.
    ///
    /// https://matrix.org/docs/spec/client_server/r0.3.0.html#put-matrix-client-r0-rooms-roomid-send-eventtype-txnid
    // TODO: Return matrix_sdk::uuid::Uuid
    pub fn get_txn_id(&self) -> String {
        let msg_str = format!("{}{}{}", self.room, self.body, self.date);
        let digest = md5::compute(msg_str.as_bytes());
        format!("{:x}", digest)
    }

    pub fn set_receipt(&mut self, receipt: HashMap<UserId, i64>) {
        self.receipt = receipt;
    }
}
