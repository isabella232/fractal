use crate::model::message::Message;
use matrix_sdk::identifiers::EventId;
use std::iter::FromIterator;
use std::slice::Iter;

/// Contains an ordered list of messages and their relations.
#[derive(Debug, Default, Clone)]
pub struct MessageList {
    messages: Vec<Message>,
}

impl MessageList {
    pub fn new() -> Self {
        Default::default()
    }

    /// Returns the message with the given event ID.
    pub fn get(&self, event_id: &EventId) -> Option<&Message> {
        self.messages
            .iter()
            .find(|m| m.id.as_ref() == Some(event_id))
    }

    /// Whether the message with the given id is in the room.
    pub fn contains(&self, msg_id: &EventId) -> bool {
        self.get(msg_id).is_some()
    }

    /// Returns an iterator over all messages.
    pub fn iter(&self) -> Iter<Message> {
        self.messages.iter()
    }

    /// Inserts the message at the correct position replacing its older version.
    pub fn add(&mut self, msg: Message) {
        assert!(msg.id.is_some());

        // Deduplication only happens for messages with the same date, so we have
        // to manually go through the message list and remove possible duplicates.
        //
        // This is necessary due to the special case of just-sent messages.
        // They don't contain the “official” timestamp from the server, but the
        // time they were sent from Fractal. Due to this circumstance, we might
        // end up with two messages having the same id, but different dates. We
        // brute-force-fix this by searching all messages for duplicates.
        self.messages.retain(|m| m.id != msg.id);

        match self.messages.binary_search(&msg) {
            Ok(idx) => self.messages[idx] = msg,
            Err(idx) => self.messages.insert(idx, msg),
        }
        // TODO: Use is_sorted (https://github.com/rust-lang/rust/issues/53485)
        // debug_assert!(self.messages.is_sorted());
    }
}

impl FromIterator<Message> for MessageList {
    fn from_iter<I: IntoIterator<Item = Message>>(messages: I) -> Self {
        let mut message_list = Self::new();
        for m in messages {
            message_list.add(m);
        }
        message_list
    }
}
