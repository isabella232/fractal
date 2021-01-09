use crate::model::message::Message;
use matrix_sdk::identifiers::EventId;
use std::collections::{HashMap, HashSet};
use std::iter;
use std::iter::FromIterator;
use std::slice::Iter;

/// Contains an ordered list of messages and their relations.
#[derive(Debug, Default, Clone)]
pub struct MessageList {
    messages: Vec<Message>,
    relating_messages: HashMap<EventId, HashSet<EventId>>,
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
        let id = msg.id.clone().unwrap();

        if msg.redacted {
            self.remove_relations(&id);
        }

        // Deduplication only happens for messages with the same date, so we have
        // to manually go through the message list and remove possible duplicates.
        //
        // This is necessary due to the special case of just-sent messages.
        // They don't contain the “official” timestamp from the server, but the
        // time they were sent from Fractal. Due to this circumstance, we might
        // end up with two messages having the same id, but different dates. We
        // brute-force-fix this by searching all messages for duplicates.
        self.messages.retain(|m| m.id != msg.id);

        if !msg.redacted {
            self.populate_relations(&msg);
        }

        match self.messages.binary_search(&msg) {
            Ok(idx) => self.messages[idx] = msg,
            Err(idx) => self.messages.insert(idx, msg),
        }
        // TODO: Use is_sorted (https://github.com/rust-lang/rust/issues/53485)
        // debug_assert!(self.messages.is_sorted());
    }

    /// Updates records of those relations the message is involved in.
    ///
    /// This updates both, relating and related, messages.
    fn populate_relations(&mut self, msg: &Message) {
        // Other messages relate to `msg`
        let id = msg.id.as_ref().cloned().unwrap();
        let relating = self.find_and_get_relating(&id);
        self.relating_messages.insert(id.clone(), relating);

        // `msg` relates to other messages
        if let Some(replace_id) = &msg.replace {
            self.update_relating(replace_id, iter::once(&id).cloned().collect());
        }
    }

    /// Remove all outgoing relations for the given event.
    fn remove_relations(&mut self, event_id: &EventId) {
        let msg = unwrap_or_unit_return!(self.get(event_id));
        let relations = msg.relations();

        let event_sets = self.relating_messages.iter_mut().filter_map(|(id, rs)| {
            if relations.contains(&id) {
                Some(rs)
            } else {
                None
            }
        });

        for set in event_sets {
            set.retain(|id| id != event_id);
        }
    }

    /// Records new messages relating to the message with the given id.
    ///
    /// This does not remove other messages relating to the given id.
    fn update_relating(&mut self, id: &EventId, relating: HashSet<EventId>) {
        let new_relating = match self.relating_messages.remove(id) {
            Some(old_relating) => old_relating.union(&relating).cloned().collect(),
            None => relating,
        };
        self.relating_messages.insert(id.clone(), new_relating);
    }

    /// Finds and returns all messages relating to the given one.
    fn find_and_get_relating(&self, id: &EventId) -> HashSet<EventId> {
        self.messages
            .iter()
            .filter(|m| m.replace.as_ref() == Some(id) || m.in_reply_to.as_ref() == Some(id))
            .map(|m| m.id.clone().unwrap())
            .collect()
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
