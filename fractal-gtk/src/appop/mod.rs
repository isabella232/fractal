use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::api::r0::AccessToken;
use matrix_sdk::identifiers::{DeviceId, RoomId, ServerName, UserId};

use gtk::prelude::*;
use matrix_sdk::Client as MatrixClient;

use crate::cache::CacheMap;

use crate::util::i18n;

use crate::model::{
    member::Member,
    room::{Room, RoomList},
};
use crate::passwd::PasswordStorage;

use crate::actions::AppState;
use crate::app::AppRuntime;
use crate::cache;
use crate::ui;
use crate::widgets;

mod about;
mod account;
pub mod attach;
mod directory;
mod invite;
mod login;
mod media_viewer;
mod member;
mod message;
mod notifications;
mod notify;
pub mod room;
mod room_settings;
mod start_chat;
pub mod state;
mod sync;
mod user;

use self::member::SearchType;
use self::message::TmpMsg;

pub type UserInfoCache = Arc<Mutex<CacheMap<UserId, (String, PathBuf)>>>;

#[derive(Clone, Debug, PartialEq)]
pub enum RoomSearchPagination {
    Initial,
    Next(String),
    NoMorePages,
}

impl From<RoomSearchPagination> for Option<String> {
    fn from(rooms_pagination: RoomSearchPagination) -> Option<String> {
        match rooms_pagination {
            RoomSearchPagination::Next(rooms_since) => Some(rooms_since),
            _ => None,
        }
    }
}

impl RoomSearchPagination {
    pub fn has_more(&self) -> bool {
        *self != RoomSearchPagination::Initial
    }
}

#[derive(Clone, Debug)]
pub struct LoginData {
    pub session_client: MatrixClient,
    pub uid: UserId,
    pub access_token: AccessToken,
    pub device_id: Box<DeviceId>,
    pub username: Option<String>,
    pub avatar: Option<PathBuf>,
    pub identity_url: Box<ServerName>,
}

pub struct AppOp {
    pub app_runtime: AppRuntime,
    pub ui: ui::UI,

    pub syncing: bool, // TODO: Replace with a Mutex
    pub msg_queue: Vec<TmpMsg>,
    pub sending_message: bool,

    pub login_data: Option<LoginData>,

    pub active_room: Option<RoomId>,
    pub join_to_room: Option<RoomId>,
    pub rooms: RoomList,
    pub room_settings: Option<widgets::RoomSettings>,
    pub history: Option<widgets::RoomHistory>,
    pub roomlist: widgets::RoomList,
    unread_rooms: usize,
    pub unsent_messages: HashMap<RoomId, (String, i32)>,
    pub typing: HashMap<RoomId, std::time::Instant>,

    pub media_viewer: Rc<RefCell<Option<widgets::MediaViewer>>>,

    pub directory_pagination: RoomSearchPagination,
    pub state: AppState,
    pub since: Option<String>,
    pub room_back_history: Rc<RefCell<Vec<AppState>>>,

    pub invitation_roomid: Option<RoomId>,
    pub md_enabled: bool,
    pub invite_list: Vec<(Member, gtk::TextChildAnchor)>,
    search_type: SearchType,

    pub directory: Vec<Room>,
    pub leaflet: libhandy::Leaflet,
    pub deck: libhandy::Deck,

    pub user_info_cache: UserInfoCache,
}

impl PasswordStorage for AppOp {}

impl AppOp {
    pub fn new(ui: ui::UI, app_runtime: AppRuntime) -> AppOp {
        let leaflet = ui
            .builder
            .get_object::<libhandy::Leaflet>("chat_page")
            .expect("Couldn't find chat_page in ui file");
        let deck = ui
            .builder
            .get_object::<libhandy::Deck>("main_deck")
            .expect("Couldn't find main_deck in ui file");

        AppOp {
            app_runtime,
            ui,
            active_room: None,
            join_to_room: None,
            rooms: HashMap::new(),
            room_settings: None,
            history: None,
            login_data: None,
            syncing: false,
            msg_queue: vec![],
            sending_message: false,
            state: AppState::Login,
            room_back_history: Rc::new(RefCell::new(vec![])),
            roomlist: widgets::RoomList::new(None, None),
            directory_pagination: RoomSearchPagination::Initial,
            unread_rooms: 0,
            since: None,
            unsent_messages: HashMap::new(),
            typing: HashMap::new(),
            media_viewer: Rc::new(RefCell::new(None)),

            md_enabled: false,
            invitation_roomid: None,
            invite_list: vec![],
            search_type: SearchType::Invite,

            directory: vec![],
            leaflet,
            deck,

            user_info_cache: Arc::new(Mutex::new(
                CacheMap::new().timeout(Duration::from_secs(60 * 60)),
            )),
        }
    }

    pub fn init(&mut self) {
        self.set_state(AppState::Loading);

        // FIXME: Username and uid should not be duplicated in cache.
        let device_id = if let Ok(data) = cache::load() {
            self.since = data.since.filter(|s| !s.is_empty());
            Some(data.device_id)
        } else {
            None
        };

        // FIXME: Storing and getting the password is insecure.
        //        Only the access token should be used.
        if let Ok((username, password, server, id_url)) = self.get_pass() {
            if let (Ok((Some(access_token), uid)), Some(dev_id)) = (self.get_token(), device_id) {
                self.bk_login(uid, access_token, dev_id, server, id_url);
            } else {
                self.connect(username, password, server, id_url);
            }
        } else {
            self.set_state(AppState::Login);
        }
    }

    fn get_window(&self) -> gtk::Window {
        self.ui
            .builder
            .get_object("main_window")
            .expect("Couldn't find main_window in ui file.")
    }

    pub fn activate(&self) {
        let window = self.get_window();
        window.show();
        window.present();
    }

    pub fn update_title(&mut self) {
        let unread = self.roomlist.rooms_with_notifications();
        if self.unread_rooms != unread {
            let window = self.get_window();
            if unread == 0 {
                window.set_title(&i18n::i18n("Fractal"));
            } else {
                // Translators: The placeholder is for the number of unread messages in the
                // application
                window.set_title(&i18n::i18n_f("Fractal [{}]", &[&unread.to_string()]));
            }
            self.unread_rooms = unread;
        }
    }

    pub fn quit(&self) {
        self.cache_rooms();
        self.disconnect();
    }

    pub fn main_menu(&self) {
        let main_menu_button = self
            .ui
            .builder
            .get_object::<gtk::MenuButton>("main_menu_button")
            .expect("Couldn't find main_menu_button in ui file.");

        main_menu_button.clicked();
    }
}
