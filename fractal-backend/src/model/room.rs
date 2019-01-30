use std::collections::HashMap;

pub use api::types::{Room, RoomMembership};
use failure::err_msg;
use failure::Error;

use rusqlite::types::ToSql;
use rusqlite::Row;

use serde_json;

use super::conn;
use super::Model;

impl Model for Room {
    fn table_name() -> &'static str {
        "room"
    }

    fn get_id(&self) -> &str {
        &self.id
    }

    fn fields() -> Vec<&'static str> {
        vec![
            "id",
            "avatar",
            "name",
            "topic",
            "alias",
            "guest_can_join",
            "world_readable",
            "n_members",
            "notifications",
            "highlight",
            "membership",
            "direct",
            "prev_batch",
            "power_levels",
        ]
    }

    fn create_sql() -> String {
        format!(
            "
        CREATE TABLE if not exists {} (
            id TEXT PRIMARY KEY,
            avatar TEXT,
            name TEXT,
            topic TEXT,
            alias TEXT,
            guest_can_join BOOLEAN NOT NULL,
            world_readable BOOLEAN NOT NULL,
            n_members NUMBER NOT NULL,
            notifications NUMBER NOT NULL,
            highlight NUMBER NOT NULL,
            membership TEXT NOT NULL,
            direct BOOLEAN NOT NULL,
            prev_batch TEXT,
            power_levels TEXT NOT NULL
        )
        ",
            Self::table_name()
        )
    }

    fn store(&self) -> Result<(), Error> {
        let fields = Self::fields().join(",");
        let questions = Self::fields()
            .iter()
            .map(|_| "?")
            .collect::<Vec<&str>>()
            .join(",");
        let query = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            Self::table_name(),
            fields,
            questions
        );

        let power_levels = serde_json::to_string(&self.power_levels)?;
        let membership = serde_json::to_string(&self.membership)?;

        conn(
            move |c| {
                c.execute(
                    &query,
                    &[
                        &self.id,
                        &self.avatar as &ToSql,
                        &self.name,
                        &self.topic,
                        &self.alias,
                        &self.guest_can_join,
                        &self.world_readable,
                        &self.n_members,
                        &self.notifications,
                        &self.highlight,
                        &membership,
                        &self.direct,
                        &self.prev_batch,
                        &power_levels,
                    ],
                )
                .map(|_| ())
                .map_err(|e| err_msg(e.to_string()))
            },
            Err(err_msg("Connection not init")),
        )
    }

    fn map_row(row: &Row) -> Self {
        let strp: String = row.get(10);
        let membership: RoomMembership = serde_json::from_str(&strp).unwrap_or_default();

        let strp: String = row.get(13);
        let power_levels: HashMap<String, i32> = serde_json::from_str(&strp).unwrap_or_default();

        Self {
            id: row.get(0),
            avatar: row.get(1),
            name: row.get(2),
            topic: row.get(3),
            alias: row.get(4),
            guest_can_join: row.get(5),
            world_readable: row.get(6),
            n_members: row.get(7),
            notifications: row.get(8),
            highlight: row.get(9),
            membership: membership,
            direct: row.get(11),
            prev_batch: row.get(12),
            power_levels: power_levels,

            messages: vec![],
            members: HashMap::new(),
        }
    }
}
