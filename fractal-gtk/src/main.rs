#![deny(dead_code, unused_imports, unused_variables)]
#[macro_use]
extern crate glib;

mod api;
mod backend;
mod client;
mod config;
mod error;
mod globals;
#[macro_use]
mod util;
mod cache;
mod model;
mod passwd;
mod uibuilder;
mod uitypes;
#[macro_use]
mod app;
mod actions;
mod widgets;

mod appop;

use std::error::Error;

use crate::app::App;
use gio::prelude::*;
use gio::ApplicationExt;

#[cfg(debug_assertions)]
use log::Level;

fn main() -> Result<(), Box<dyn Error>> {
    let clap_app = clap::App::new("fractal")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Matrix group messaging app")
        .arg(
            clap::Arg::with_name("v")
                .short("v")
                .multiple(true)
                .help("Sets the level of verbosity"),
        );

    let clap_args = clap_app.get_matches();
    #[cfg(debug_assertions)]
    {
        if clap_args.occurrences_of("v") == 0 {
            loggerv::init_with_level(Level::Info).expect("Failed to initialize logger");
        } else {
            loggerv::init_with_verbosity(clap_args.occurrences_of("v"))
                .expect("Failed to initialize logger");
        }
    }
    #[cfg(not(debug_assertions))]
    {
        loggerv::init_with_verbosity(clap_args.occurrences_of("v"))
            .expect("Failed to initialize logger");
    }

    let res = gio::Resource::load(config::PKGDATADIR.to_owned() + "/resources.gresource")
        .expect("Could not load gresource file");
    gio::resources_register(&res);

    // Initialize GStreamer. This checks, among other things, what plugins are available
    gst::init()?;

    // Create a Application with default flags
    let application = gtk::Application::new(Some(config::APP_ID), gio::ApplicationFlags::empty())?;

    application.set_resource_base_path(Some("/org/gnome/Fractal"));

    application.connect_startup(|application| {
        App::on_startup(application);
    });

    application.run(&[]);

    Ok(())
}
