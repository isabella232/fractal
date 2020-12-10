use super::UI;
use crate::appop::member::SearchType;
use crate::appop::UserInfoCache;
use crate::cache::download_to_cache;
use crate::globals;
use crate::model::member::Member;
use crate::util::i18n::{i18n, i18n_k};
use crate::widgets::{self, AvatarExt};
use gtk::prelude::*;
use matrix_sdk::identifiers::UserId;
use matrix_sdk::Client as MatrixClient;

impl UI {
    pub fn add_to_invite(
        &mut self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        member: Member,
        search_type: SearchType,
    ) {
        if self.invite_list.iter().any(|(mem, _)| *mem == member) {
            return;
        }

        let textviewid = match search_type {
            SearchType::Invite => "invite_entry",
            SearchType::DirectChat => "to_chat_entry",
        };

        let invite_entry = self
            .builder
            .get_object::<gtk::TextView>(textviewid)
            .expect("Can't find invite_entry in ui file.");

        if let SearchType::DirectChat = search_type {
            self.invite_list = vec![];

            if let Some(buffer) = invite_entry.get_buffer() {
                let mut start = buffer.get_start_iter();
                let mut end = buffer.get_end_iter();

                buffer.delete(&mut start, &mut end);
            }
        }

        self.direct_chat_dialog.button.set_sensitive(true);

        if let Some(btn) = self.builder.get_object::<gtk::Button>("invite_button") {
            btn.set_sensitive(true)
        }

        if let Some(buffer) = invite_entry.get_buffer() {
            let mut start_word = buffer.get_iter_at_offset(buffer.get_property_cursor_position());
            let mut end_word = buffer.get_iter_at_offset(buffer.get_property_cursor_position());

            // Remove the search input in the entry before inserting the member's pill
            if !start_word.starts_word() {
                start_word.backward_word_start();
            }
            if !end_word.ends_word() {
                end_word.forward_word_end();
            }
            buffer.delete(&mut start_word, &mut end_word);

            if let Some(anchor) = buffer.create_child_anchor(&mut end_word) {
                let w = build_memberbox_pill(session_client, user_info_cache, member.clone());
                invite_entry.add_child_at_anchor(&w, &anchor);
                self.invite_list.push((member, anchor));
            }
        }
    }

    pub fn rm_from_invite(&mut self, uid: UserId, search_type: SearchType) {
        let idx = self.invite_list.iter().position(|x| x.0.uid == uid);
        if let Some(i) = idx {
            self.invite_list.remove(i);
        }

        if self.invite_list.is_empty() {
            if let Some(btn) = self.builder.get_object::<gtk::Button>("direct_chat_button") {
                btn.set_sensitive(false)
            }

            if let Some(btn) = self.builder.get_object::<gtk::Button>("invite_button") {
                btn.set_sensitive(false)
            }
        }

        let dialogid = match search_type {
            SearchType::Invite => "invite_user_dialog",
            SearchType::DirectChat => "direct_chat_dialog",
        };

        let dialog = self
            .builder
            .get_object::<gtk::Dialog>(dialogid)
            .expect("Can’t find invite_user_dialog in ui file.");

        dialog.resize(300, 200);
    }

    pub fn detect_removed_invite(&mut self, search_type: SearchType) {
        let invite_list: Vec<_> = self
            .invite_list
            .iter()
            .map(|(member, anchor)| (member.uid.clone(), anchor.clone()))
            .collect();

        for (uid, anchor) in invite_list {
            if anchor.get_deleted() {
                self.rm_from_invite(uid, search_type);
            }
        }
    }

    pub fn show_invite_user_dialog(&mut self, room_name: Option<&str>) {
        let dialog = self
            .builder
            .get_object::<gtk::Dialog>("invite_user_dialog")
            .expect("Can't find invite_user_dialog in ui file.");
        let scroll = self
            .builder
            .get_object::<gtk::Widget>("user_search_scroll")
            .expect("Can't find user_search_scroll in ui file.");
        let headerbar = self
            .builder
            .get_object::<gtk::HeaderBar>("invite_headerbar")
            .expect("Can't find invite_headerbar in ui file.");

        if let Some(name) = room_name {
            headerbar.set_title(Some(i18n_k("Invite to {name}", &[("name", name)]).as_str()));
        } else {
            headerbar.set_title(Some(i18n("Invite").as_str()));
        }

        self.set_invite_user_dialog_placeholder(SearchType::Invite);

        dialog.present();
        scroll.hide();
    }

    pub fn close_invite_dialog(&mut self) {
        let listbox = self
            .builder
            .get_object::<gtk::ListBox>("user_search_box")
            .expect("Can't find user_search_box in ui file.");
        let scroll = self
            .builder
            .get_object::<gtk::Widget>("user_search_scroll")
            .expect("Can't find user_search_scroll in ui file.");
        let invite_entry = self
            .builder
            .get_object::<gtk::TextView>("invite_entry")
            .expect("Can't find invite_entry in ui file.");
        let dialog = self
            .builder
            .get_object::<gtk::Dialog>("invite_user_dialog")
            .expect("Can't find invite_user_dialog in ui file.");

        self.invite_list = vec![];
        if let Some(buffer) = invite_entry.get_buffer() {
            let mut start = buffer.get_start_iter();
            let mut end = buffer.get_end_iter();

            buffer.delete(&mut start, &mut end);
        }
        for ch in listbox.get_children().iter() {
            listbox.remove(ch);
        }
        scroll.hide();
        dialog.hide();
        dialog.resize(300, 200);
    }

    pub fn show_inv_dialog(&self, sender: Option<&Member>, room_name: Option<&str>) {
        let dialog = self
            .builder
            .get_object::<gtk::MessageDialog>("invite_dialog")
            .expect("Can't find invite_dialog in ui file.");

        let empty = String::new();
        let room_name = room_name.unwrap_or(&empty);
        let title = i18n_k("Join {room_name}?", &[("room_name", &room_name)]);
        let secondary = if let Some(ref sender) = sender {
            let sender_name = sender.get_alias();
            i18n_k(
                "You’ve been invited to join <b>{room_name}</b> room by <b>{sender_name}</b>",
                &[("room_name", &room_name), ("sender_name", &sender_name)],
            )
        } else {
            i18n_k(
                "You’ve been invited to join <b>{room_name}</b>",
                &[("room_name", &room_name)],
            )
        };

        dialog.set_property_text(Some(title.as_str()));
        dialog.set_property_secondary_use_markup(true);
        dialog.set_property_secondary_text(Some(secondary.as_str()));

        dialog.present();
    }

    pub fn set_invite_user_dialog_placeholder(&mut self, search_type: SearchType) {
        let textviewid = match search_type {
            SearchType::Invite => "invite_entry",
            SearchType::DirectChat => "to_chat_entry",
        };

        let invite_entry = self
            .builder
            .get_object::<gtk::TextView>(textviewid)
            .expect("Can't find invite_entry in ui file.");

        if let Some(buffer) = invite_entry.get_buffer() {
            let start = buffer.get_start_iter();
            let end = buffer.get_end_iter();

            if let Some(text) = buffer.get_text(&start, &end, true) {
                if text.is_empty() && self.invite_list.is_empty() {
                    buffer.set_text(globals::PLACEHOLDER_TEXT);

                    let start = buffer.get_start_iter();
                    let end = buffer.get_end_iter();

                    buffer.apply_tag_by_name("placeholder", &start, &end);
                }
            }
        }
    }

    pub fn remove_invite_user_dialog_placeholder(&mut self, search_type: SearchType) {
        let textviewid = match search_type {
            SearchType::Invite => "invite_entry",
            SearchType::DirectChat => "to_chat_entry",
        };

        let invite_entry = self
            .builder
            .get_object::<gtk::TextView>(textviewid)
            .expect("Can't find invite_entry in ui file.");

        if let Some(buffer) = invite_entry.get_buffer() {
            let start = buffer.get_start_iter();
            let end = buffer.get_end_iter();

            if let Some(text) = buffer.get_text(&start, &end, true) {
                if text == globals::PLACEHOLDER_TEXT && self.invite_list.is_empty() {
                    buffer.set_text("");

                    let start = buffer.get_start_iter();
                    let end = buffer.get_end_iter();

                    buffer.remove_tag_by_name("placeholder", &start, &end);
                }
            }
        }
    }
}

fn build_memberbox_pill(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    member: Member,
) -> gtk::Box {
    let pill = gtk::Box::new(gtk::Orientation::Horizontal, 3);

    let username = gtk::Label::new(None);

    username.set_text(&member.get_alias());
    username.set_margin_end(3);
    username.get_style_context().add_class("msg-highlighted");

    let avatar = widgets::Avatar::avatar_new(Some(globals::PILL_ICON_SIZE));
    let data = avatar.circle(
        member.uid.to_string(),
        Some(member.get_alias()),
        globals::PILL_ICON_SIZE,
        None,
        None,
    );

    download_to_cache(session_client, user_info_cache, member.uid, data);

    avatar.set_margin_start(3);

    pill.pack_start(&avatar, true, true, 0);
    pill.pack_start(&username, true, true, 0);
    pill.show_all();
    pill
}
