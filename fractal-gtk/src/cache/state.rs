use lazy_static::lazy_static;
use mdl::Cache;
use mdl::Model;
use serde::{Deserialize, Serialize};

use anyhow::{anyhow, Error};

use std::fs::remove_dir_all;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::util::cache_dir_path;
use matrix_sdk::identifiers::{DeviceId, UserId};

// Models

/// The application state, here we store the current state
/// information like the username, the last sync "since"
/// param and that kind of information
#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub since: Option<String>,
    pub username: String,
    pub uid: UserId,
    pub device_id: Box<DeviceId>,
}

impl Model for AppState {
    fn key(&self) -> String {
        "state".to_string()
    }
}

// Cache
#[derive(Clone)]
pub struct FCache {
    cache: Arc<Mutex<Option<Cache>>>,
}

impl FCache {
    fn get_store(&self) -> MutexGuard<Option<Cache>> {
        let mut guard = self.cache.lock().unwrap();
        if guard.is_none() {
            let maybe_db_path = cache_dir_path(None, "cache.mdl").ok();
            let db: String = maybe_db_path
                .and_then(|p| p.to_str().map(Into::into))
                .expect("Fatal error: Can't start the cache");
            // TODO: Replace Cache with another library. Not expecting a proper
            //       Path type for the path of the DB is bonkers.
            let mdl_cache = Cache::new(&db).expect("Fatal error: Can't start the cache");
            *guard = Some(mdl_cache);
        }
        guard
    }

    pub fn destroy(&self) -> Result<(), Error> {
        let mut guard = self.cache.lock().unwrap();
        guard.take();

        let fname = cache_dir_path(None, "cache.mdl")
            .or_else(|_| Err(anyhow!("Can't remove cache file")))?;
        remove_dir_all(fname).or_else(|_| Err(anyhow!("Can't remove cache file")))
    }

    pub fn get_st(&self) -> Result<AppState, Error> {
        let cache = self.get_store();
        AppState::get(cache.as_ref().unwrap(), "state")
    }

    pub fn save_st(&self, st: AppState) -> Result<(), Error> {
        let cache = self.get_store();
        st.store(cache.as_ref().unwrap())?;

        Ok(())
    }
}

// The cache object, it's the same for the whole process
lazy_static! {
    static ref CACHE: FCache = FCache {
        cache: Arc::new(Mutex::new(None))
    };
}

pub fn get() -> FCache {
    CACHE.clone()
}
