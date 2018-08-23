use std::process::Command;
use std::env;
use std::fs::File;
use std::path::Path;
use std::io::Write;

fn main() {
    // Compile Gresource
    let out = Command::new("glib-compile-resources")
        .args(&["--generate", "resources.xml"])
        .current_dir("res")
        .status()
        .expect("failed to generate resources");
    assert!(out.success());

    // Generating build globals
    let default_locales = "./fractal-gtk/po".to_string();
    let default_app_id = "org.gnome.Fractal".to_string();
    let default_name_suffix = "".to_string();
    let default_version = "".to_string();

    let localedir = env::var("FRACTAL_LOCALEDIR").unwrap_or(default_locales);
    let app_id = env::var("FRACTAL_APP_ID").unwrap_or(default_app_id);
    let name_suffix = env::var("FRACTAL_NAME_SUFFIX").unwrap_or(default_name_suffix);
    let version = env::var("FRACTAL_VERSION").unwrap_or(default_version);

    let out_dir = env::var("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("build_globals.rs");
    let mut f = File::create(&dest_path).unwrap();

    let globals = format!("
pub static LOCALEDIR: &'static str = \"{}\";
pub static APP_ID: &'static str = \"{}\";
pub static NAME_SUFFIX: &'static str = \"{}\";
pub static VERSION: &'static str = \"{}\";
",
        localedir, app_id, name_suffix, version);

    f.write_all(&globals.into_bytes()[..]).unwrap();
}
