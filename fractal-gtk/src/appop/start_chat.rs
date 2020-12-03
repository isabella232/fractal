use crate::app::RUNTIME;
use crate::appop::AppOp;
use crate::appop::SearchType;
use crate::backend::room;
use crate::backend::HandleError;

impl AppOp {
    pub fn start_chat(&mut self) {
        if self.ui.invite_list.len() != 1 {
            return;
        }

        let (session_client, user_id) = unwrap_or_unit_return!(self
            .login_data
            .as_ref()
            .map(|ld| (ld.session_client.clone(), ld.uid.clone())));
        let member = self.ui.invite_list[0].0.clone();

        RUNTIME.spawn(async move {
            match room::direct_chat(session_client, &user_id, member).await {
                Ok(r) => {
                    APPOP!(new_room, (r));
                }
                Err(err) => {
                    err.handle_error();
                }
            }
        });

        self.ui.close_direct_chat_dialog();
    }

    pub fn show_direct_chat_dialog(&mut self) {
        self.search_type = SearchType::DirectChat;
        self.ui.show_direct_chat_dialog();
    }
}
