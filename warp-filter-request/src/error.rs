use core::fmt;

use warp::{hyper::http::Error as HyperHttpError, reject::Reject};

//
#[derive(Debug)]
pub enum Error {
    HyperHttpError(HyperHttpError),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

//
impl From<HyperHttpError> for Error {
    fn from(err: HyperHttpError) -> Self {
        Self::HyperHttpError(err)
    }
}

//
impl Reject for Error {}
