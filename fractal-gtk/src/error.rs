#[derive(Debug)]
pub enum Error {
    GlibError(glib::Error),
    ReqwestError(fractal_api::reqwest::Error),
}

impl From<fractal_api::reqwest::Error> for Error {
    fn from(err: fractal_api::reqwest::Error) -> Error {
        Error::ReqwestError(err)
    }
}

impl From<glib::Error> for Error {
    fn from(err: glib::Error) -> Error {
        Error::GlibError(err)
    }
}
