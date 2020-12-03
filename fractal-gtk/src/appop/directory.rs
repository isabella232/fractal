use super::RoomSearchPagination;
use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::backend::{directory, HandleError};
use crate::model::room::Room;
use matrix_sdk::directory::RoomNetwork;
use matrix_sdk::thirdparty::ProtocolInstance;

impl AppOp {
    pub fn init_protocols(&self) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        RUNTIME.spawn(async move {
            match directory::protocols(session_client).await {
                Ok(protocols) => {
                    APPOP!(set_protocols, (protocols));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }

    pub fn set_protocols(&self, protocols: Vec<ProtocolInstance>) {
        self.ui.set_protocols(protocols);
    }

    pub fn search_rooms(&mut self) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));

        let (protocol, homeserver, search_term) = self
            .ui
            .get_search_rooms_query(self.directory_pagination.clone());

        if let RoomSearchPagination::NoMorePages = self.directory_pagination {
            // there are no more rooms. We don't need to request for more
            return;
        }

        let rooms_since: Option<String> = self.directory_pagination.clone().into();
        RUNTIME.spawn(async move {
            let query = directory::room_search(
                session_client,
                homeserver.as_deref(),
                search_term.as_deref(),
                protocol
                    .as_deref()
                    .map_or(RoomNetwork::Matrix, RoomNetwork::ThirdParty),
                rooms_since.as_deref(),
            )
            .await;

            match query {
                Ok((rooms, rooms_since)) => {
                    APPOP!(append_directory_rooms, (rooms, rooms_since));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });
    }

    #[inline]
    pub fn load_more_rooms(&mut self) {
        self.search_rooms();
    }

    pub fn append_directory_rooms(&mut self, mut rooms: Vec<Room>, rooms_since: Option<String>) {
        let session_client =
            unwrap_or_unit_return!(self.login_data.as_ref().map(|ld| ld.session_client.clone()));
        rooms.sort_by_key(|a| -i128::from(a.n_members));
        self.directory_pagination = rooms_since
            .map(RoomSearchPagination::Next)
            .unwrap_or(RoomSearchPagination::NoMorePages);

        self.ui.append_directory_rooms(rooms, session_client);
    }

    pub fn reset_directory_state(&self) {
        self.ui.reset_directory_state();
    }
}
