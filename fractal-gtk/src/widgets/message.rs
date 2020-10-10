use crate::appop::UserInfoCache;
use crate::cache::download_to_cache;
use crate::globals;
use crate::ui::MessageContent as Message;
use crate::ui::RowType;
use crate::util::i18n::i18n;
use crate::util::markup_text;
use crate::widgets;
use crate::widgets::message_menu::MessageMenu;
use crate::widgets::AvatarExt;
use crate::widgets::ClipContainer;
use crate::widgets::{AudioPlayerWidget, PlayerExt, VideoPlayerWidget};
use anyhow::Context;
use chrono::prelude::*;
use either::Either;
use glib::clone;
use gtk::{prelude::*, ButtonExt, ContainerExt, LabelExt, Overlay, WidgetExt};
use html2pango::block::{HtmlBlock, markup_html};
use itertools::Itertools;
use matrix_sdk::Client as MatrixClient;
use std::cmp::max;
use std::rc::Rc;

// A message row in the room history
#[derive(Clone, Debug)]
pub struct MessageBox {
    container: MessageBoxContainer,
    msg_widget: MessageBoxMsg,
}

impl MessageBox {
    // create the message row with or without a header
    pub fn create(
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        msg: &Message,
        has_header: bool,
        is_temp: bool,
    ) -> Self {
        let container = MessageBoxContainer::new();

        container.set_msg_styles(msg.mtype);
        let msg_widget = match msg.mtype {
            RowType::Video if is_temp => MessageBoxMsg::tmpwidget("Uploading video."),
            RowType::Audio if is_temp => MessageBoxMsg::tmpwidget("Uploading audio."),
            RowType::Image if is_temp => MessageBoxMsg::tmpwidget("Uploading image."),
            RowType::File if is_temp => MessageBoxMsg::tmpwidget("Uploading file."),
            RowType::Emote => {
                container.root.set_margin_top(12);
                MessageBoxMsg::small_widget(&container, session_client, msg)
            }
            _ if has_header => {
                container.root.set_margin_top(12);
                MessageBoxMsg::widget(&container, session_client, user_info_cache, msg)
            }
            _ => MessageBoxMsg::small_widget(&container, session_client, msg),
        };

        if is_temp {
            container.root.get_style_context().add_class("msg-tmp");
        }

        container.eventbox.add(msg_widget.root());
        container.root.show_all();
        container.connect_right_click_menu(msg, None);

        Self {
            container,
            msg_widget,
        }
    }

    pub fn create_tmp(
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        msg: &Message,
    ) -> Self {
        Self::create(session_client, user_info_cache, msg, true, true)
    }

    pub fn update_header(
        &mut self,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        msg: Message,
        has_header: bool,
    ) {
        let msg_widget = if has_header && msg.mtype != RowType::Emote {
            self.container.root.set_margin_top(12);
            MessageBoxMsg::widget(&self.container, session_client, user_info_cache, &msg)
        } else {
            if let RowType::Emote = msg.mtype {
                self.container.root.set_margin_top(12);
            }
            MessageBoxMsg::small_widget(&self.container, session_client, &msg)
        };
        if let Some(eb) = self.container.eventbox.get_child() {
            self.container.eventbox.remove(&eb);
        }
        self.container.eventbox.add(msg_widget.root());
        self.container.root.show_all();
    }

    pub fn get_widget(&self) -> &gtk::ListBoxRow {
        &self.container.root
    }

    pub fn get_video_player(&self) -> Option<&Rc<VideoPlayerWidget>> {
        match &self.msg_widget {
            MessageBoxMsg::Final { content, .. } => {
                if let MessageBodyType::Video(ref player) = content.body_bx.type_extras {
                    player.as_ref()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn has_header(&self) -> bool {
        match &self.msg_widget {
            MessageBoxMsg::Final { content, .. } => content.info.is_some(),
            _ => false,
        }
    }
}

#[derive(Clone, Debug)]
struct MessageBoxContainer {
    root: gtk::ListBoxRow,
    eventbox: gtk::EventBox,
    gesture: gtk::GestureLongPress,
}

impl MessageBoxContainer {
    fn new() -> Self {
        let eventbox = gtk::EventBox::new();

        let root = gtk::ListBoxRow::new();
        root.add(&eventbox);
        root.set_selectable(false);

        let gesture = gtk::GestureLongPress::new(&eventbox);
        gesture.set_propagation_phase(gtk::PropagationPhase::Capture);
        gesture.set_touch_only(true);

        Self {
            root,
            eventbox,
            gesture,
        }
    }

    // Add classes to the widget based on message type
    fn set_msg_styles(&self, mtype: RowType) {
        let style = self.root.get_style_context();
        match mtype {
            RowType::Mention => style.add_class("msg-mention"),
            RowType::Emote => style.add_class("msg-emote"),
            RowType::Emoji => style.add_class("msg-emoji"),
            _ => {}
        }
    }

    fn connect_media_viewer(&self, msg: &Message) -> Option<()> {
        let evid = msg.msg.id.as_ref()?.to_string();
        let data = glib::Variant::from(evid);
        self.root.set_action_name(Some("app.open-media-viewer"));
        self.root.set_action_target_value(Some(&data));
        None
    }

    fn connect_right_click_menu(&self, msg: &Message, label: Option<&gtk::Label>) -> Option<()> {
        let mtype = msg.mtype;
        let redactable = msg.redactable;
        let widget = if let Some(l) = label {
            l.upcast_ref::<gtk::Widget>()
        } else {
            self.eventbox.upcast_ref::<gtk::Widget>()
        };

        let id = msg.msg.id.clone();
        widget.connect_button_press_event(move |w, e| {
            if e.triggers_context_menu() {
                let menu = MessageMenu::new(id.as_ref(), &mtype, &redactable, Some(w));
                let coords = e.get_position();
                menu.show_at_coords(w, coords);
                Inhibit(true)
            } else {
                Inhibit(false)
            }
        });

        let id = msg.msg.id.clone();
        self.gesture
            .connect_pressed(clone!(@weak widget => move |_, x, y| {
                let menu = MessageMenu::new(id.as_ref(), &mtype, &redactable, Some(&widget));
                menu.show_at_coords(&widget, (x, y));
            }));
        None
    }
}

#[derive(Clone, Debug)]
enum MessageBoxMsg {
    Temp(gtk::Box),
    Final {
        root: gtk::Box,
        avatar: Option<widgets::Avatar>,
        content: MessageBoxContent,
    },
}

impl MessageBoxMsg {
    fn tmpwidget(label_content: &str) -> Self {
        let upload_attachment_msg = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        upload_attachment_msg.add(&gtk::Label::new(Some(i18n(label_content).as_str())));

        Self::Temp(upload_attachment_msg)
    }

    fn widget(
        container: &MessageBoxContainer,
        session_client: MatrixClient,
        user_info_cache: UserInfoCache,
        msg: &Message,
    ) -> Self {
        // msg
        // +--------+---------+
        // | avatar | content |
        // +--------+---------+
        let msg_widget = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let content = MessageBoxContent::build(container, session_client.clone(), msg, true);
        // TODO: make build_room_msg_avatar() faster (currently ~1ms)
        let avatar = build_room_msg_avatar(session_client, user_info_cache, msg);

        msg_widget.pack_start(&avatar, false, false, 0);
        msg_widget.pack_start(&content.root, true, true, 0);

        Self::Final {
            root: msg_widget,
            avatar: Some(avatar),
            content,
        }
    }

    fn small_widget(
        container: &MessageBoxContainer,
        session_client: MatrixClient,
        msg: &Message,
    ) -> Self {
        // msg
        // +--------+---------+
        // |        | content |
        // +--------+---------+
        let msg_widget = gtk::Box::new(gtk::Orientation::Horizontal, 5);
        let content = MessageBoxContent::build(container, session_client, msg, false);
        content.root.set_margin_start(50);

        msg_widget.pack_start(&content.root, true, true, 0);

        Self::Final {
            root: msg_widget,
            avatar: None,
            content,
        }
    }

    fn root(&self) -> &gtk::Box {
        match self {
            Self::Temp(root) => root,
            Self::Final { root, .. } => root,
        }
    }
}

#[derive(Clone, Debug)]
struct MessageBoxContent {
    root: gtk::Box,
    info: Option<MessageBoxInfoHeader>,
    body_bx: MessageBodyBox,
}

impl MessageBoxContent {
    fn build(
        container: &MessageBoxContainer,
        session_client: MatrixClient,
        msg: &Message,
        info_header: bool,
    ) -> Self {
        // content
        // +---------+
        // | info    |
        // +---------+
        // | body_bx |
        // +---------+
        let content = gtk::Box::new(gtk::Orientation::Vertical, 0);

        let info = if info_header {
            let info = MessageBoxInfoHeader::from(msg);
            info.root.set_margin_top(2);
            info.root.set_margin_bottom(3);
            content.pack_start(&info.root, false, false, 0);

            Some(info)
        } else {
            None
        };

        let body_bx = MessageBodyBox::build(&container, session_client, msg);
        content.pack_start(&body_bx.root, true, true, 0);

        Self {
            root: content,
            info,
            body_bx,
        }
    }
}

fn build_room_msg_avatar(
    session_client: MatrixClient,
    user_info_cache: UserInfoCache,
    msg: &Message,
) -> widgets::Avatar {
    let uid = msg.msg.sender.clone();
    let alias = msg.sender_name.clone();
    let avatar = widgets::Avatar::avatar_new(Some(globals::MSG_ICON_SIZE));
    avatar.set_valign(gtk::Align::Start);

    let data = avatar.circle(
        uid.to_string(),
        alias.clone(),
        globals::MSG_ICON_SIZE,
        None,
        None,
    );

    download_to_cache(
        session_client.clone(),
        user_info_cache,
        uid.clone(),
        data.clone(),
    );

    avatar
}

fn set_label_styles(w: &gtk::Label) {
    w.set_line_wrap(true);
    w.set_line_wrap_mode(pango::WrapMode::WordChar);
    w.set_justify(gtk::Justification::Left);
    w.set_xalign(0.0);
    w.set_valign(gtk::Align::Start);
    w.set_halign(gtk::Align::Fill);
    w.set_selectable(true);
}

fn highlight_username(
    label: gtk::Label,
    attr: &pango::AttrList,
    alias: &str,
    input: String,
) -> Option<()> {
    fn contains((start, end): (i32, i32), item: i32) -> bool {
        if start <= end {
            start <= item && end > item
        } else {
            start <= item || end > item
        }
    }

    let mut input = input.to_lowercase();
    let bounds = label.get_selection_bounds();
    let context = label.get_style_context();
    let fg = context.lookup_color("theme_selected_bg_color")?;
    let red = fg.red * 65535. + 0.5;
    let green = fg.green * 65535. + 0.5;
    let blue = fg.blue * 65535. + 0.5;
    let color = pango::Attribute::new_foreground(red as u16, green as u16, blue as u16)?;

    let alias = &alias.to_lowercase();
    let mut removed_char = 0;
    while input.contains(alias) {
        let pos = {
            let start = input.find(alias)? as i32;
            (start, start + alias.len() as i32)
        };
        let mut color = color.clone();
        let mark_start = removed_char as i32 + pos.0;
        let mark_end = removed_char as i32 + pos.1;
        let mut final_pos = Some((mark_start, mark_end));
        // exclude selected text
        if let Some((bounds_start, bounds_end)) = bounds {
            // If the selection is within the alias
            if contains((mark_start, mark_end), bounds_start)
                && contains((mark_start, mark_end), bounds_end)
            {
                final_pos = Some((mark_start, bounds_start));
                // Add blue color after a selection
                let mut color = color.clone();
                color.set_start_index(bounds_end as u32);
                color.set_end_index(mark_end as u32);
                attr.insert(color);
            } else {
                // The alias starts inside a selection
                if contains(bounds?, mark_start) {
                    final_pos = Some((bounds_end, final_pos?.1));
                }
                // The alias ends inside a selection
                if contains(bounds?, mark_end - 1) {
                    final_pos = Some((final_pos?.0, bounds_start));
                }
            }
        }

        if let Some((start, end)) = final_pos {
            color.set_start_index(start as u32);
            color.set_end_index(end as u32);
            attr.insert(color);
        }
        {
            let end = pos.1 as usize;
            input.drain(0..end);
        }
        removed_char += pos.1 as u32;
    }

    None
}

#[derive(Clone, Debug)]
enum MessageBodyType {
    Sticker,
    Audio,
    Image(Option<widgets::image::Image>), // gtk::DrawingArea
    Video(Option<Rc<VideoPlayerWidget>>),
    Emote(gtk::Label),
    File,
    Text,
}

#[derive(Clone, Debug)]
struct MessageBodyBox {
    root: gtk::Box,
    body: gtk::Box,
    edit_mark: Option<gtk::Image>,
    type_extras: MessageBodyType,
}

impl MessageBodyBox {
    fn build(container: &MessageBoxContainer, session_client: MatrixClient, msg: &Message) -> Self {
        // body_bx
        // +------+-----------+
        // | body | edit_mark |
        // +------+-----------+
        let body_bx = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let (body, type_extras) = build_room_msg(container, session_client, msg);

        body_bx.pack_start(&body, true, true, 0);

        let edit_mark = if let Some(replace_date) = msg.msg.replace_date() {
            let edit_mark =
                gtk::Image::from_icon_name(Some("document-edit-symbolic"), gtk::IconSize::Button);
            edit_mark.get_style_context().add_class("edit-mark");
            edit_mark.set_valign(gtk::Align::End);

            let edit_tooltip = replace_date.format(&i18n("Last edited %c")).to_string();
            edit_mark.set_tooltip_text(Some(&edit_tooltip));

            body_bx.pack_start(&edit_mark, false, false, 0);

            Some(edit_mark)
        } else {
            None
        };

        Self {
            root: body_bx,
            body,
            edit_mark,
            type_extras,
        }
    }
}

type BodyAndType = (gtk::Box, MessageBodyType);

fn build_room_msg(
    container: &MessageBoxContainer,
    session_client: MatrixClient,
    msg: &Message,
) -> BodyAndType {
    let (body, type_extras) = match msg.mtype {
        RowType::Sticker => build_room_msg_sticker(session_client, msg),
        RowType::Audio => build_room_audio_player(session_client, msg),
        RowType::Image => build_room_msg_image(session_client, msg),
        RowType::Video => build_room_video_player(session_client, msg),
        RowType::Emote => build_room_msg_emote(msg),
        RowType::File => build_room_msg_file(msg),
        _ => build_room_msg_body(container, msg),
    };

    match type_extras {
        MessageBodyType::Image(Some(_)) => {
            container.connect_media_viewer(msg);
        }
        MessageBodyType::Video(Some(_)) => {
            container.connect_media_viewer(msg);
        }
        MessageBodyType::Emote(ref msg_label) => {
            container.connect_right_click_menu(msg, Some(msg_label));
        }
        _ => {}
    }

    (body, type_extras)
}

fn build_room_msg_body_html(_container: &MessageBoxContainer, msg: &Message) -> anyhow::Result<gtk::Box> {
    let raw = msg.msg.formatted_body.clone().unwrap_or_default();

    let blocks =
        markup_html(&raw).with_context(|| format!("Could not render message: {}", &raw))?;
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 6);
    for b in blocks {
        let widget = render_html_block(&b);
        bx.add(&widget);
    }
    Ok(bx)
}

fn render_html_block(block: &HtmlBlock) -> gtk::Widget {
    match block {
        HtmlBlock::Heading(n, s) => {
            let w = gtk::Label::new(None);
            set_label_styles(&w);
            w.set_markup(&s);
            w.get_style_context().add_class(&format!("h{}", n));
            w.upcast::<gtk::Widget>()
        }
        HtmlBlock::UList(elements) => {
            let w = gtk::Label::new(None);
            set_label_styles(&w);

            let text = elements
                .iter()
                .map(|li| format!(" • {}", li))
                .collect::<Vec<String>>()
                .join("\n");
            w.set_markup(&text);

            w.upcast::<gtk::Widget>()
        }
        HtmlBlock::OList(elements) => {
            let w = gtk::Label::new(None);
            set_label_styles(&w);

            let text = elements
                .iter()
                .enumerate()
                .map(|(i, li)| format!(" {}. {}", i + 1, li))
                .collect::<Vec<String>>()
                .join("\n");

            w.set_markup(&text);

            w.upcast::<gtk::Widget>()
        }
        HtmlBlock::Code(s) => {
            let buffer = sourceview4::Buffer::new::<gtk::TextTagTable>(None);
            buffer.set_text(&s);
            let view = sourceview4::View::with_buffer(&buffer);
            view.set_editable(false);
            view.set_wrap_mode(gtk::WrapMode::WordChar);
            view.get_style_context().add_class("codeview");
            view.upcast::<gtk::Widget>()
        }
        HtmlBlock::Quote(blocks) => {
            let bx = gtk::Box::new(gtk::Orientation::Vertical, 6);
            bx.get_style_context().add_class("quote");
            for b in blocks.iter() {
                let w = render_html_block(&b);
                bx.add(&w);
            }
            bx.upcast::<gtk::Widget>()
        }
        HtmlBlock::Text(s) => {
            let w = gtk::Label::new(None);
            set_label_styles(&w);
            w.set_markup(&s);
            w.upcast::<gtk::Widget>()
        }
    }
}

fn build_room_msg_sticker(session_client: MatrixClient, msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    if let Some(url) = msg.msg.url.clone() {
        let image = widgets::image::Image::new(Either::Left(url))
            .size(Some(globals::MAX_STICKER_SIZE))
            .build(session_client);
        image.widget.set_tooltip_text(Some(&msg.msg.body[..]));

        bx.add(&image.widget);
    }

    (bx, MessageBodyType::Sticker)
}

fn build_room_audio_player(session_client: MatrixClient, msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 6);

    if let Some(url) = msg.msg.url.clone() {
        let player = AudioPlayerWidget::new();
        let start_playing = false;
        PlayerExt::initialize_stream(
            player.clone(),
            session_client,
            url,
            bx.clone(),
            start_playing,
        );

        let control_box = PlayerExt::get_controls_container(&player)
            .expect("Every AudioPlayer must have controls.");
        bx.pack_start(&control_box, false, true, 0);
    }

    let download_btn =
        gtk::Button::from_icon_name(Some("document-save-symbolic"), gtk::IconSize::Button);
    download_btn.set_tooltip_text(Some(i18n("Save").as_str()));

    let evid = msg
        .msg
        .id
        .as_ref()
        .map(|evid| evid.to_string())
        .unwrap_or_default();
    let data = glib::Variant::from(evid);
    download_btn.set_action_target_value(Some(&data));
    download_btn.set_action_name(Some("message.save_as"));
    bx.pack_start(&download_btn, false, false, 3);

    let outer_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
    let file_name = gtk::Label::new(Some(&format!("<b>{}</b>", msg.msg.body)));
    file_name.set_use_markup(true);
    file_name.set_xalign(0.0);
    file_name.set_line_wrap(true);
    file_name.set_line_wrap_mode(pango::WrapMode::WordChar);
    outer_box.pack_start(&file_name, false, false, 0);
    outer_box.pack_start(&bx, false, false, 0);
    outer_box.get_style_context().add_class("audio-box");

    (outer_box, MessageBodyType::Audio)
}

fn build_room_msg_image(session_client: MatrixClient, msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    // If the thumbnail is not a valid URL we use the msg.url
    let img = msg
        .msg
        .thumb
        .clone()
        .filter(|m| m.scheme() == "mxc" || m.scheme().starts_with("http"))
        .or_else(|| msg.msg.url.clone())
        .map(Either::Left)
        .or_else(|| Some(Either::Right(msg.msg.local_path.clone()?)));

    let image = if let Some(img_path) = img {
        let image = widgets::image::Image::new(img_path)
            .size(Some(globals::MAX_IMAGE_SIZE))
            .build(session_client);

        image.widget.get_style_context().add_class("image-widget");

        bx.pack_start(&image.widget, true, true, 0);
        bx.show_all();

        Some(image)
    } else {
        None
    };

    (bx, MessageBodyType::Image(image))
}

fn build_room_video_player(session_client: MatrixClient, msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let player = if let Some(url) = msg.msg.url.clone() {
        let with_controls = false;
        let player = VideoPlayerWidget::new(with_controls);
        let start_playing = false;
        PlayerExt::initialize_stream(
            player.clone(),
            session_client,
            url,
            bx.clone(),
            start_playing,
        );

        let overlay = Overlay::new();
        let video_widget = player.get_video_widget();
        video_widget.set_size_request(-1, 390);
        VideoPlayerWidget::auto_adjust_video_dimensions(&player);
        overlay.add(&video_widget);

        let play_button = gtk::Button::new();
        let play_icon = gtk::Image::from_icon_name(
            Some("media-playback-start-symbolic"),
            gtk::IconSize::Dialog,
        );
        play_button.set_image(Some(&play_icon));
        play_button.set_halign(gtk::Align::Center);
        play_button.set_valign(gtk::Align::Center);
        play_button.get_style_context().add_class("osd");
        play_button.get_style_context().add_class("play-icon");
        play_button.get_style_context().add_class("flat");
        let evid = msg
            .msg
            .id
            .as_ref()
            .map(|evid| evid.to_string())
            .unwrap_or_default();
        let data = glib::Variant::from(evid);
        play_button.set_action_name(Some("app.open-media-viewer"));
        play_button.set_action_target_value(Some(&data));
        overlay.add_overlay(&play_button);

        let menu_button = gtk::MenuButton::new();
        let three_dot_icon =
            gtk::Image::from_icon_name(Some("view-more-symbolic"), gtk::IconSize::Button);
        menu_button.set_image(Some(&three_dot_icon));
        menu_button.get_style_context().add_class("osd");
        menu_button.get_style_context().add_class("round-button");
        menu_button.get_style_context().add_class("flat");
        menu_button.set_margin_top(12);
        menu_button.set_margin_end(12);
        menu_button.set_opacity(0.8);
        menu_button.set_halign(gtk::Align::End);
        menu_button.set_valign(gtk::Align::Start);
        menu_button.connect_size_allocate(|button, allocation| {
            let diameter = max(allocation.width, allocation.height);
            button.set_size_request(diameter, diameter);
        });
        overlay.add_overlay(&menu_button);

        let evid = msg.msg.id.as_ref();
        let redactable = msg.redactable;
        let menu = MessageMenu::new(evid, &RowType::Video, &redactable, None);
        menu_button.set_popover(Some(&menu.get_popover()));

        let clip_container = ClipContainer::new();
        clip_container.add(&overlay);

        bx.pack_start(&clip_container, true, true, 0);

        Some(player)
    } else {
        None
    };

    (bx, MessageBodyType::Video(player))
}

fn build_room_msg_emote(msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    // Use MXID till we have a alias
    let sname = msg
        .sender_name
        .clone()
        .unwrap_or_else(|| msg.msg.sender.to_string());
    let msg_label = gtk::Label::new(None);
    let markup = markup_text(&msg.msg.body);

    msg_label.set_markup(&format!("<b>{}</b> {}", sname, markup));
    set_label_styles(&msg_label);

    bx.add(&msg_label);

    (bx, MessageBodyType::Emote(msg_label))
}

fn build_room_msg_file(msg: &Message) -> BodyAndType {
    let bx = gtk::Box::new(gtk::Orientation::Horizontal, 12);
    let btn_bx = gtk::Box::new(gtk::Orientation::Horizontal, 0);

    let name = msg.msg.body.as_str();
    let name_lbl = gtk::Label::new(Some(name));
    name_lbl.set_tooltip_text(Some(name));
    name_lbl.set_ellipsize(pango::EllipsizeMode::End);

    name_lbl.get_style_context().add_class("msg-highlighted");

    let download_btn =
        gtk::Button::from_icon_name(Some("document-save-symbolic"), gtk::IconSize::Button);
    download_btn.set_tooltip_text(Some(i18n("Save").as_str()));

    let evid = msg
        .msg
        .id
        .as_ref()
        .map(|evid| evid.to_string())
        .unwrap_or_default();

    let data = glib::Variant::from(&evid);
    download_btn.set_action_target_value(Some(&data));
    download_btn.set_action_name(Some("message.save_as"));

    let open_btn =
        gtk::Button::from_icon_name(Some("document-open-symbolic"), gtk::IconSize::Button);
    open_btn.set_tooltip_text(Some(i18n("Open").as_str()));

    let data = glib::Variant::from(&evid);
    open_btn.set_action_target_value(Some(&data));
    open_btn.set_action_name(Some("message.open_with"));

    btn_bx.pack_start(&open_btn, false, false, 0);
    btn_bx.pack_start(&download_btn, false, false, 0);
    btn_bx.get_style_context().add_class("linked");

    bx.pack_start(&name_lbl, false, false, 0);
    bx.pack_start(&btn_bx, false, false, 0);

    (bx, MessageBodyType::File)
}

fn build_room_msg_body(container: &MessageBoxContainer, msg: &Message) -> BodyAndType {
    let bx = match msg.msg.format.as_deref() {
        Some("org.matrix.custom.html") =>
            build_room_msg_body_html(container, &msg)
            .unwrap_or_else(|_err| build_room_msg_body_text(container, &msg)),
        _ => build_room_msg_body_text(container, &msg),
    };
    (bx, MessageBodyType::Text)
}

fn build_room_msg_body_text(container: &MessageBoxContainer, msg: &Message) -> gtk::Box {
    let bx = gtk::Box::new(gtk::Orientation::Vertical, 6);

    let msgs_by_kind_of_line = msg.msg.body.lines().group_by(|&line| kind_of_line(line));
    let msg_parts = msgs_by_kind_of_line.into_iter().map(|(k, group)| {
        let mut v: Vec<&str> = if k == MsgPartType::Quote {
            group.map(trim_start_quote).collect()
        } else {
            group.collect()
        };
        // We need to remove the first and last empty line (if any) because quotes use \n\n
        if v.starts_with(&[""]) {
            v.drain(..1);
        }
        if v.ends_with(&[""]) {
            v.pop();
        }
        let part = v.join("\n");

        let part_widget = gtk::Label::new(None);
        part_widget.set_markup(&markup_text(&part));
        set_label_styles(&part_widget);

        if k == MsgPartType::Quote {
            part_widget.get_style_context().add_class("quote");
        }

        part_widget
    });

    for part in msg_parts {
        if msg.mtype == RowType::Mention {
            let highlights = msg.highlights.clone();
            part.connect_property_cursor_position_notify(move |w| {
                let attr = pango::AttrList::new();
                for light in highlights.clone() {
                    highlight_username(w.clone(), &attr, &light, w.get_text().to_string());
                }
                w.set_attributes(Some(&attr));
            });

            let highlights = msg.highlights.clone();
            part.connect_property_selection_bound_notify(move |w| {
                let attr = pango::AttrList::new();
                for light in highlights.clone() {
                    highlight_username(w.clone(), &attr, &light, w.get_text().to_string());
                }
                w.set_attributes(Some(&attr));
            });

            let attr = pango::AttrList::new();
            for light in msg.highlights.iter() {
                highlight_username(part.clone(), &attr, light, part.get_text().to_string());
            }
            part.set_attributes(Some(&attr));
        }

        container.connect_right_click_menu(msg, Some(&part));
        bx.add(&part);
    }

    bx
}

#[derive(Clone, Debug)]
pub struct MessageBoxInfoHeader {
    root: gtk::Box,
    username_event_box: gtk::EventBox,
    username: gtk::Label,
    date: gtk::Label,
}

impl From<&Message> for MessageBoxInfoHeader {
    fn from(msg: &Message) -> Self {
        // info
        // +----------+------+
        // | username | date |
        // +----------+------+
        let root = gtk::Box::new(gtk::Orientation::Horizontal, 0);

        let username = build_room_msg_username(
            msg.sender_name
                .as_deref()
                .unwrap_or(msg.msg.sender.as_str()),
        );
        let date = build_room_msg_date(&msg.msg.date);

        let username_event_box = gtk::EventBox::new();
        username_event_box.add(&username);

        root.pack_start(&username_event_box, true, true, 0);
        root.pack_start(&date, false, false, 0);

        Self {
            root,
            username_event_box,
            username,
            date,
        }
    }
}

fn build_room_msg_username(uname: &str) -> gtk::Label {
    let username = gtk::Label::new(Some(uname));

    username.set_ellipsize(pango::EllipsizeMode::End);
    username.set_justify(gtk::Justification::Left);
    username.set_halign(gtk::Align::Start);
    username.get_style_context().add_class("username");

    username
}

fn build_room_msg_date(dt: &DateTime<Local>) -> gtk::Label {
    // TODO: get system preference for 12h/24h
    let use_ampm = false;
    let format = if use_ampm {
        // Use 12h time format (AM/PM)
        i18n("%l∶%M %p")
    } else {
        // Use 24 time format
        i18n("%R")
    };

    let d = dt.format(&format).to_string();

    let date = gtk::Label::new(None);
    date.set_markup(&format!("<span alpha=\"60%\">{}</span>", d.trim()));
    date.set_line_wrap(true);
    date.set_justify(gtk::Justification::Right);
    date.set_valign(gtk::Align::Start);
    date.set_halign(gtk::Align::End);
    date.get_style_context().add_class("timestamp");

    date
}

#[derive(PartialEq)]
enum MsgPartType {
    Normal,
    Quote,
}

fn kind_of_line(line: &str) -> MsgPartType {
    if line.trim_start().starts_with('>') {
        MsgPartType::Quote
    } else {
        MsgPartType::Normal
    }
}

fn trim_start_quote(line: &str) -> &str {
    line.trim_start().get(1..).unwrap_or(line).trim_start()
}
