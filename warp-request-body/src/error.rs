use core::fmt;

use warp::{hyper::Error as HyperError, Error as WarpError};

//
#[derive(Debug)]
pub enum Error {
    WarpError(WarpError),
    HyperError(HyperError),
}
impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

//
impl From<WarpError> for Error {
    fn from(err: WarpError) -> Self {
        Self::WarpError(err)
    }
}

//
impl From<HyperError> for Error {
    fn from(err: HyperError) -> Self {
        Self::HyperError(err)
    }
}
