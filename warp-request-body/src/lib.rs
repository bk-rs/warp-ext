use core::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use pin_project_lite::pin_project;
use warp::{Buf, Error as WarpError, Stream};

pub mod utils;

//
pin_project! {
    #[project = BodyProj]
    pub enum Body {
        Buf { inner: Box<dyn Buf> },
        Bytes { inner: Bytes },
        Stream { #[pin] inner: Pin<Box<dyn Stream<Item = Result<Bytes, WarpError>> + Send + 'static>> },
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buf { inner: _ } => write!(f, "Buf"),
            Self::Bytes { inner: _ } => write!(f, "Bytes"),
            Self::Stream { inner: _ } => write!(f, "Stream"),
        }
    }
}

impl Default for Body {
    fn default() -> Self {
        Self::Bytes {
            inner: Bytes::default(),
        }
    }
}

//
impl Body {
    pub fn with_buf(inner: impl Buf + 'static) -> Self {
        Self::Buf {
            inner: Box::new(inner),
        }
    }

    pub fn with_bytes(inner: Bytes) -> Self {
        Self::Bytes { inner }
    }

    pub fn with_stream(
        inner: impl Stream<Item = Result<impl Buf + 'static, WarpError>> + Send + 'static,
    ) -> Self {
        Self::Stream {
            inner: Box::pin(utils::buf_stream_to_bytes_stream(inner)),
        }
    }

    pub fn require_to_bytes_async(&self) -> bool {
        matches!(self, Self::Stream { inner: _ })
    }

    pub fn to_bytes(self) -> Bytes {
        match self {
            Self::Buf { inner } => utils::buf_to_bytes(inner),
            Self::Bytes { inner } => inner,
            Self::Stream { inner: _ } => panic!("Please call require_to_bytes_async first"),
        }
    }

    pub async fn to_bytes_async(self) -> Result<Bytes, WarpError> {
        match self {
            Self::Buf { inner } => Ok(utils::buf_to_bytes(inner)),
            Self::Bytes { inner } => Ok(inner),
            Self::Stream { inner } => utils::bytes_stream_to_bytes(inner).await,
        }
    }
}

//
impl Stream for Body {
    type Item = Result<Bytes, WarpError>;

    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match self.project() {
            BodyProj::Buf { inner: buf } => {
                if buf.has_remaining() {
                    let bytes = Bytes::copy_from_slice(buf.chunk());
                    let cnt = buf.chunk().len();
                    buf.advance(cnt);
                    Poll::Ready(Some(Ok(bytes)))
                } else {
                    Poll::Ready(None)
                }
            }
            BodyProj::Bytes { inner } => {
                if !inner.is_empty() {
                    let bytes = inner.clone();
                    inner.clear();
                    Poll::Ready(Some(Ok(bytes)))
                } else {
                    Poll::Ready(None)
                }
            }
            BodyProj::Stream { inner } => inner.poll_next(cx),
        }
    }
}
