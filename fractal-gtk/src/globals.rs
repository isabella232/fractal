pub static INITIAL_MESSAGES: usize = 40;
pub static CACHE_SIZE: usize = 40;
pub static MSG_ICON_SIZE: i32 = 40;
pub static USERLIST_ICON_SIZE: i32 = 30;
pub static MINUTES_TO_SPLIT_MSGS: i64 = 30;
pub static DEFAULT_HOMESERVER: &'static str = "https://matrix.org";
pub static DEFAULT_IDENTITYSERVER: &'static str = "https://vector.im";

pub static MAX_IMAGE_SIZE: (i32, i32) = (600, 400);
pub static MAX_STICKER_SIZE: (i32, i32) = (200, 130);

include!(concat!(env!("OUT_DIR"), "/build_globals.rs"));
