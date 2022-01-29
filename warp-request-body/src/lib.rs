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
    pub fn with_buf(buf: impl Buf + 'static) -> Self {
        Self::Buf {
            inner: Box::new(buf),
        }
    }

    pub fn with_bytes(bytes: Bytes) -> Self {
        Self::Bytes { inner: bytes }
    }

    pub fn with_stream(
        stream: impl Stream<Item = Result<impl Buf + 'static, WarpError>> + Send + 'static,
    ) -> Self {
        Self::Stream {
            inner: Box::pin(utils::buf_stream_to_bytes_stream(stream)),
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

#[cfg(test)]
mod tests {
    use futures_util::StreamExt as _;

    use super::*;

    #[tokio::test]
    async fn test_with_buf() {
        //
        let buf = warp::test::request()
            .body("foo")
            .filter(&warp::body::aggregate())
            .await
            .unwrap();
        let body = Body::with_buf(buf);
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_aggregate())
            .await
            .unwrap();
        let (_, buf) = req.into_parts();
        let body = Body::with_buf(buf);
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));

        //
        let buf = warp::test::request()
            .body("foo")
            .filter(&warp::body::aggregate())
            .await
            .unwrap();
        let mut body = Body::with_buf(buf);
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
        assert!(body.next().await.is_none());
    }

    #[tokio::test]
    async fn test_with_bytes() {
        //
        let bytes = warp::test::request()
            .body("foo")
            .filter(&warp::body::bytes())
            .await
            .unwrap();
        let body = Body::with_bytes(bytes);
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_bytes())
            .await
            .unwrap();
        let (_, bytes) = req.into_parts();
        let body = Body::with_bytes(bytes);
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));

        //
        let bytes = warp::test::request()
            .body("foo")
            .filter(&warp::body::bytes())
            .await
            .unwrap();
        let mut body = Body::with_bytes(bytes);
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
        assert!(body.next().await.is_none());
    }

    #[tokio::test]
    async fn test_with_stream() {
        //
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();
        let body = Body::with_stream(stream);
        assert!(body.require_to_bytes_async());
        assert_eq!(
            body.to_bytes_async().await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_stream())
            .await
            .unwrap();
        let (_, stream) = req.into_parts();
        let body = Body::with_stream(stream);
        assert!(body.require_to_bytes_async());
        assert_eq!(
            body.to_bytes_async().await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );

        //
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();
        let mut body = Body::with_stream(stream);
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
        assert!(body.next().await.is_none());
    }
}
