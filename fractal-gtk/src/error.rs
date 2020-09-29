#[derive(Debug)]
pub enum Error {
    GlibError(glib::Error),
    ReqwestError(matrix_sdk::reqwest::Error),
}

impl From<matrix_sdk::reqwest::Error> for Error {
    fn from(err: matrix_sdk::reqwest::Error) -> Error {
        Error::ReqwestError(err)
    }
}

impl From<glib::Error> for Error {
    fn from(err: glib::Error) -> Error {
        Error::GlibError(err)
    }
}
