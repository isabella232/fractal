use gettextrs::{bindtextdomain, setlocale, textdomain, LocaleCategory};
use gio::prelude::*;
use glib::clone;
use gtk::prelude::*;
use lazy_static::lazy_static;
use libhandy::prelude::*;
use tokio::runtime::Runtime as TokioRuntime;

use log::error;

use crate::appop::AppOp;

use crate::actions;
use crate::config;
use crate::ui;
use crate::widgets;

mod windowstate;

use windowstate::WindowState;

static mut APP_RUNTIME: Option<AppRuntime> = None;

lazy_static! {
    pub static ref RUNTIME: TokioRuntime = TokioRuntime::new().unwrap();
}

#[macro_export]
macro_rules! APPOP {
    ($fn: ident, ($($x:ident),*) ) => {{
        $( let $x = $x.clone(); )*
        crate::app::get_app_runtime().update_state_with(move |op| {
            op.$fn($($x),*);
        });
    }};
    ($fn: ident) => {{
        APPOP!($fn, ( ) );
    }}
}

#[derive(Clone)]
pub struct AppRuntime(glib::Sender<Box<dyn FnOnce(&mut AppOp)>>);

impl AppRuntime {
    fn init(ui: ui::UI) -> Self {
        let (app_tx, app_rx) = glib::MainContext::channel(Default::default());
        let app_runtime = Self(app_tx);
        let mut state = AppOp::new(ui, app_runtime.clone());

        unsafe {
            APP_RUNTIME = Some(app_runtime.clone());
        }

        app_rx.attach(None, move |update_state| {
            update_state(&mut state);

            glib::Continue(true)
        });

        app_runtime
    }

    pub fn update_state_with(&self, update_fn: impl FnOnce(&mut AppOp) + 'static) {
        let _ = self.0.send(Box::new(update_fn));
    }
}

fn new(gtk_app: gtk::Application) -> (AppRuntime, ui::UI) {
    // Set up the textdomain for gettext
    setlocale(LocaleCategory::LcAll, "");
    bindtextdomain("fractal", config::LOCALEDIR);
    textdomain("fractal");

    glib::set_application_name("fractal");
    glib::set_prgname(Some("fractal"));

    // Add style provider
    let provider = gtk::CssProvider::new();
    provider.load_from_resource("/org/gnome/Fractal/app.css");
    gtk::StyleContext::add_provider_for_screen(
        &gdk::Screen::get_default().expect("Error initializing gtk css provider."),
        &provider,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );

    let ui = ui::UI::new(gtk_app.clone());
    let app_runtime = AppRuntime::init(ui.clone());

    let settings: gio::Settings = gio::Settings::new("org.gnome.Fractal");
    let window_state = WindowState::load_from_gsettings(&settings);
    ui.main_window
        .set_default_size(window_state.width, window_state.height);
    if window_state.is_maximized {
        ui.main_window.maximize();
    } else if window_state.x > 0 && window_state.y > 0 {
        ui.main_window.move_(window_state.x, window_state.y);
    }
    ui.main_window.show_all();

    if gtk_app
        .get_application_id()
        .map_or(false, |s| s.ends_with("Devel"))
    {
        ui.main_window.get_style_context().add_class("devel");
    }

    let leaflet = ui
        .builder
        .get_object::<libhandy::Leaflet>("chat_page")
        .expect("Can't find chat_page in ui file.");
    let container = ui
        .builder
        .get_object::<gtk::Box>("history_container")
        .expect("Can't find history_container in ui file.");
    let popover = ui
        .builder
        .get_object::<gtk::Popover>("autocomplete_popover")
        .expect("Can't find autocomplete_popover in ui file.");

    if leaflet.get_folded() {
        container.get_style_context().add_class("folded-history");
        popover.get_style_context().add_class("narrow");
    }

    leaflet.connect_property_folded_notify(clone!(@weak container => move |leaflet| {
        if leaflet.get_folded() {
            container.get_style_context().add_class("folded-history");
            popover.get_style_context().add_class("narrow");
        } else {
            container.get_style_context().remove_class("folded-history");
            popover.get_style_context().remove_class("narrow");
        }
    }));

    let view_stack = ui
        .builder
        .get_object::<gtk::Stack>("subview_stack")
        .expect("Can't find subview_stack in ui file.");

    /* Add account settings view to the view stack */
    let child = ui
        .builder
        .get_object::<gtk::Box>("account_settings_box")
        .expect("Can't find account_settings_box in ui file.");
    view_stack.add_named(&child, "account-settings");

    let main_stack = ui
        .builder
        .get_object::<gtk::Stack>("main_content_stack")
        .expect("Can't find main_content_stack in ui file.");

    // Add login view to the main stack
    let login = widgets::LoginWidget::new(app_runtime.clone());
    main_stack.add_named(&login.container, "login");

    app_runtime.update_state_with(|state| {
        state
            .ui
            .gtk_app
            .set_accels_for_action("login.back", &["Escape"]);
        actions::Global::new(state);
        state.ui.connect_gtk(state.app_runtime.clone());
    });

    (app_runtime, ui)
}

pub fn on_startup(gtk_app: &gtk::Application) {
    // Create application.
    let (app_runtime, ui) = new(gtk_app.clone());

    // Initialize libhandy
    libhandy::init();

    gtk_app.connect_activate(clone!(@strong app_runtime => move |_| {
        app_runtime.update_state_with(|state| {
            on_activate(&state.ui);
        });
    }));

    ui.main_window.connect_property_has_toplevel_focus_notify(
        clone!(@strong app_runtime => move |_| {
            app_runtime.update_state_with(|state| {
                state.mark_active_room_messages();
            });
        }),
    );

    ui.main_window.connect_delete_event(move |window, _| {
        let settings: gio::Settings = gio::Settings::new("org.gnome.Fractal");
        let w = window.upcast_ref();
        let window_state = WindowState::from_window(w);
        if let Err(err) = window_state.save_in_gsettings(&settings) {
            error!("Can't save the window settings: {:?}", err);
        }
        Inhibit(false)
    });

    app_runtime.update_state_with(|state| {
        state.init();
    });

    // When the application is shut down we drop our app struct
    gtk_app.connect_shutdown(move |_| {
        app_runtime.update_state_with(|state| {
            on_shutdown(state);
        });
    });
}

fn on_activate(ui: &ui::UI) {
    ui.main_window.show();
    ui.main_window.present();
}

fn on_shutdown(appop: &AppOp) {
    appop.quit();
}

pub fn get_app_runtime() -> &'static AppRuntime {
    unsafe {
        APP_RUNTIME
            .as_ref()
            .expect("Fatal: AppRuntime has not been initialized")
    }
}
