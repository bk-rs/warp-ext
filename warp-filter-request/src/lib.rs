/// [Ref](https://github.com/seanmonstar/warp/issues/139)
///
/// Note: Cannot upgrade, becase warp::filters::ext::optional require Clone, bug hyper::upgrade::OnUpgrade cannot Clone
///
pub mod error;

pub(crate) mod common;
mod with_body_aggregate;
mod with_body_bytes;
mod with_body_stream;

pub use self::with_body_aggregate::{
    with_body_aggregate, with_body_aggregate_and_one_extension,
    with_body_aggregate_and_two_extensions,
};
pub use self::with_body_bytes::{
    with_body_bytes, with_body_bytes_and_one_extension, with_body_bytes_and_two_extensions,
};
pub use self::with_body_stream::{
    with_body_stream, with_body_stream_and_one_extension, with_body_stream_and_two_extensions,
};
