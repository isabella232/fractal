use super::UI;
use crate::app::RUNTIME;
use crate::appop::RoomSearchPagination;
use crate::backend::room;
use crate::backend::HandleError;
use crate::model::room::Room;
use crate::util::i18n::i18n;
use crate::util::markup_text;
use crate::widgets::{self, AvatarExt};
use crate::APPOP;
use gtk::prelude::*;
use gtk::WidgetExt;
use matrix_sdk::identifiers::{RoomAliasId, RoomIdOrAliasId};
use matrix_sdk::thirdparty::ProtocolInstance;
use matrix_sdk::Client as MatrixClient;

const AVATAR_SIZE: i32 = 60;
const JOIN_BUTTON_WIDTH: i32 = 84;

impl UI {
    pub fn set_protocols(&self, protocols: Vec<ProtocolInstance>) {
        let combo = self
            .builder
            .get_object::<gtk::ListStore>("protocol_model")
            .expect("Can't find protocol_model in ui file.");
        combo.clear();

        for p in protocols {
            combo.insert_with_values(None, &[0, 1], &[&p.desc, &p.network_id]);
        }
    }

    pub fn get_search_rooms_query(
        &self,
        directory_pagination: RoomSearchPagination,
    ) -> (Option<String>, Option<String>, Option<String>) {
        let other_protocol_radio = self
            .builder
            .get_object::<gtk::RadioButton>("other_protocol_radio")
            .expect("Can't find other_protocol_radio in ui file.");

        let protocol: Option<String> = if other_protocol_radio.get_active() {
            let protocol_combo = self
                .builder
                .get_object::<gtk::ComboBox>("protocol_combo")
                .expect("Can't find protocol_combo in ui file.");

            let protocol_model = self
                .builder
                .get_object::<gtk::ListStore>("protocol_model")
                .expect("Can't find protocol_model in ui file.");

            let active = protocol_combo.get_active().map_or(-1, |uint| uint as i32);

            protocol_model
                .iter_nth_child(None, active)
                .and_then(|it| protocol_model.get_value(&it, 1).get().ok()?)
        } else {
            None
        };

        let other_homeserver_radio = self
            .builder
            .get_object::<gtk::RadioButton>("other_homeserver_radio")
            .expect("Can't find other_homeserver_radio in ui file.");

        let other_homeserver_url = self
            .builder
            .get_object::<gtk::EntryBuffer>("other_homeserver_url")
            .expect("Can't find other_homeserver_url in ui file.");

        let homeserver = if other_homeserver_radio.get_active() {
            Some(other_homeserver_url.get_text())
        } else {
            None
        };

        let q = self
            .builder
            .get_object::<gtk::Entry>("directory_search_entry")
            .expect("Can't find directory_search_entry in ui file.");

        if !directory_pagination.has_more() {
            let directory = self
                .builder
                .get_object::<gtk::ListBox>("directory_room_list")
                .expect("Can't find directory_room_list in ui file.");
            for ch in directory.get_children() {
                directory.remove(&ch);
            }

            let directory_stack = self
                .builder
                .get_object::<gtk::Stack>("directory_stack")
                .expect("Can't find directory_stack in ui file.");
            let directory_spinner = self
                .builder
                .get_object::<gtk::Box>("directory_spinner")
                .expect("Can't find directory_spinner in ui file.");
            directory_stack.set_visible_child(&directory_spinner);

            q.set_sensitive(false);
        }

        let search_term = Some(q.get_text().to_string()).filter(|s| !s.is_empty());

        (protocol, homeserver, search_term)
    }

    pub fn append_directory_rooms(&mut self, rooms: Vec<Room>, session_client: MatrixClient) {
        let directory = self
            .builder
            .get_object::<gtk::ListBox>("directory_room_list")
            .expect("Can't find directory_room_list in ui file.");
        directory.get_style_context().add_class("room-directory");

        let directory_stack = self
            .builder
            .get_object::<gtk::Stack>("directory_stack")
            .expect("Can't find directory_stack in ui file.");
        let directory_clamp = self
            .builder
            .get_object::<libhandy::Clamp>("directory_clamp")
            .expect("Can't find directory_clamp in ui file.");
        directory_stack.set_visible_child(&directory_clamp);

        for r in rooms.iter() {
            let room_widget = build_room_box_widget(&r, session_client.clone());
            directory.add(&room_widget);
        }

        let q = self
            .builder
            .get_object::<gtk::Entry>("directory_search_entry")
            .expect("Can't find directory_search_entry in ui file.");
        q.set_sensitive(true);
    }

    pub fn reset_directory_state(&self) {
        let q = self
            .builder
            .get_object::<gtk::Entry>("directory_search_entry")
            .expect("Can't find directory_search_entry in ui file.");
        q.set_sensitive(true);

        let directory_stack = self
            .builder
            .get_object::<gtk::Stack>("directory_stack")
            .expect("Can't find directory_stack in ui file.");
        let directory_clamp = self
            .builder
            .get_object::<libhandy::Clamp>("directory_clamp")
            .expect("Can't find directory_clamp in ui file.");
        directory_stack.set_visible_child(&directory_clamp);
    }
}

fn build_room_box_widget(room: &Room, session_client: MatrixClient) -> gtk::ListBoxRow {
    let row = gtk::ListBoxRow::new();
    let room_box = build_room_box(room, session_client);

    row.set_selectable(false);
    row.add(&room_box);
    row.show_all();

    row
}

fn build_room_box(room: &Room, session_client: MatrixClient) -> gtk::Box {
    let widget_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let avatar = widgets::Avatar::avatar_new(Some(AVATAR_SIZE));
    avatar.circle(
        room.id.to_string(),
        room.name.clone(),
        AVATAR_SIZE,
        None,
        None,
    );
    widget_box.pack_start(&avatar, false, false, 18);

    let details_box = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let name = room
        .name
        .as_ref()
        .filter(|n| !n.is_empty())
        .map(String::as_str)
        .or(room.alias.as_ref().map(RoomAliasId::as_str))
        .unwrap_or_default();

    let name_label = gtk::Label::new(None);
    name_label.set_line_wrap(true);
    name_label.set_line_wrap_mode(pango::WrapMode::WordChar);
    name_label.set_markup(&format!("<b>{}</b>", markup_text(name)));
    name_label.set_justify(gtk::Justification::Left);
    name_label.set_halign(gtk::Align::Start);
    name_label.set_valign(gtk::Align::Start);
    name_label.set_xalign(0.0);

    let topic_label = gtk::Label::new(None);
    if !room.topic.clone().unwrap_or_default().is_empty() {
        topic_label.set_line_wrap(true);
        topic_label.set_line_wrap_mode(pango::WrapMode::WordChar);
        topic_label.set_lines(2);
        topic_label.set_ellipsize(pango::EllipsizeMode::End);
        topic_label.set_markup(&markup_text(&room.topic.clone().unwrap_or_default()));
        topic_label.set_justify(gtk::Justification::Left);
        topic_label.set_halign(gtk::Align::Start);
        topic_label.set_valign(gtk::Align::Start);
        topic_label.set_xalign(0.0);
    }

    let alias_label = gtk::Label::new(None);
    alias_label.set_line_wrap(true);
    alias_label.set_line_wrap_mode(pango::WrapMode::WordChar);
    alias_label.set_markup(&format!(
        "<span alpha=\"60%\">{}</span>",
        room.alias
            .as_ref()
            .map(RoomAliasId::as_str)
            .unwrap_or_default()
    ));
    alias_label.set_justify(gtk::Justification::Left);
    alias_label.set_halign(gtk::Align::Start);
    alias_label.set_valign(gtk::Align::Start);
    alias_label.set_xalign(0.0);

    details_box.add(&name_label);
    if !topic_label.get_text().to_string().is_empty() {
        details_box.add(&topic_label);
    }
    details_box.add(&alias_label);

    let membership_grid = gtk::Grid::new();
    membership_grid.set_column_spacing(6);

    let members_icon =
        gtk::Image::from_icon_name(Some("system-users-symbolic"), gtk::IconSize::Menu);
    members_icon.get_style_context().add_class("dim-label");

    let members_count = gtk::Label::new(Some(&format!("{}", room.n_members)[..]));
    members_count.get_style_context().add_class("dim-label");

    let join_button = gtk::Button::with_label(i18n("Join").as_str());
    let room_id_or_alias: RoomIdOrAliasId = room.id.clone().into();
    join_button.connect_clicked(move |_| {
        let session_client = session_client.clone();
        let room_id_or_alias = room_id_or_alias.clone();
        RUNTIME.spawn(async move {
            match room::join_room(session_client, &room_id_or_alias).await {
                Ok(jtr) => {
                    let jtr = Some(jtr);
                    APPOP!(set_join_to_room, (jtr));
                    APPOP!(reload_rooms);
                }
                Err(err) => err.handle_error(),
            }
        });
    });
    join_button.set_property_width_request(JOIN_BUTTON_WIDTH);

    membership_grid.attach(&join_button, 0, 0, 4, 1);
    membership_grid.attach(&members_icon, 5, 0, 1, 1);
    membership_grid.attach(&members_count, 6, 0, 1, 1);

    details_box.add(&membership_grid);

    widget_box.pack_start(&details_box, true, true, 0);

    widget_box.show_all();

    widget_box
}
