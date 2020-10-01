use crate::app::RUNTIME;
use crate::appop::UserInfoCache;
use crate::backend::user;
use crate::util::cache_dir_path;
use gtk::LabelExt;
use matrix_sdk::Client as MatrixClient;

use anyhow::Error;
use matrix_sdk::identifiers::{DeviceId, UserId};
use std::collections::HashMap;
use std::hash::Hash;
use std::time::{Duration, Instant};

use crate::widgets::AvatarData;
use std::cell::RefCell;
use std::rc::Rc;

mod state;
pub use self::state::get;
pub use self::state::AppState;
pub use self::state::FCache;

// user info cache, uid -> (name, avatar)
#[derive(Clone, Debug)]
pub struct CacheMap<K: Clone + Eq + Hash, V: Clone> {
    map: HashMap<K, (Instant, V)>,
    timeout: Duration,
}

impl<K: Clone + Eq + Hash, V: Clone> CacheMap<K, V> {
    pub fn new() -> Self {
        CacheMap {
            map: HashMap::new(),
            timeout: Duration::from_secs(10),
        }
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn get(&self, k: &K) -> Option<&V> {
        match self.map.get(k) {
            Some(t) => {
                if t.0.elapsed() >= self.timeout {
                    return None;
                }
                Some(&t.1)
            }
            None => None,
        }
    }

    pub fn insert(&mut self, k: K, v: V) {
        let now = Instant::now();
        self.map.insert(k, (now, v));
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        self.map.remove(k).map(|v| v.1)
    }
}

pub struct CacheData {
    pub since: Option<String>,
    pub username: String,
    pub uid: UserId,
    pub device_id: Box<DeviceId>,
}

pub fn store(
    since: Option<String>,
    username: String,
    uid: UserId,
    device_id: Box<DeviceId>,
) -> Result<(), Error> {
    let st = AppState {
        since,
        username,
        uid,
        device_id,
    };

    get().save_st(st)?;

    Ok(())
}

pub fn load() -> Result<CacheData, Error> {
    let st = get().get_st()?;

    Ok(CacheData {
        since: st.since,
        username: st.username,
        uid: st.uid,
        device_id: st.device_id,
    })
}

pub fn remove_from_cache(user_info_cache: UserInfoCache, user_id: &UserId) {
    user_info_cache.lock().unwrap().remove(&user_id);
    if let Ok(dest) = cache_dir_path(None, &user_id.to_string()) {
        let _ = std::fs::remove_file(dest);
    }
}

/// this downloads a avatar and stores it in the cache folder
pub fn download_to_cache(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    uid: UserId,
    data: Rc<RefCell<AvatarData>>,
) {
    let response = RUNTIME.spawn(user::get_user_info(session_client, user_info_cache, uid));

    glib::MainContext::default().spawn_local(async move {
        if let Ok(_) = response.await {
            data.borrow_mut().redraw(None);
        }
    });
}

/* Get username based on the MXID, we should cache the username */
pub fn download_to_cache_username(
    session_client: MatrixClient,
    uid: UserId,
    label: gtk::Label,
    avatar: Option<Rc<RefCell<AvatarData>>>,
) {
    let response = RUNTIME.spawn(async move {
        user::get_username(session_client, &uid)
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    });
    glib::MainContext::default().spawn_local(async move {
        if let Ok(username) = response.await {
            label.set_text(&username);
            if let Some(ref rc_data) = avatar {
                let mut data = rc_data.borrow_mut();
                data.redraw(Some(username));
            }
        }
    });
}

/* Download username for a given MXID and update a emote message
 * FIXME: We should cache this request and do it before we need to display the username in an emote*/
pub fn download_to_cache_username_emote(
    session_client: MatrixClient,
    uid: UserId,
    text: &str,
    label: gtk::Label,
    avatar: Option<Rc<RefCell<AvatarData>>>,
) {
    let response = RUNTIME.spawn(async move {
        user::get_username(session_client, &uid)
            .await
            .ok()
            .flatten()
            .unwrap_or_default()
    });
    let text = text.to_string();
    glib::MainContext::default().spawn_local(async move {
        if let Ok(username) = response.await {
            label.set_markup(&format!("<b>{}</b> {}", &username, text));
            if let Some(ref rc_data) = avatar {
                let mut data = rc_data.borrow_mut();
                data.redraw(Some(username));
            }
        }
    });
}
