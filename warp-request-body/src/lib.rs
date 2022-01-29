use core::{
    fmt,
    pin::Pin,
    task::{Context, Poll},
};

use bytes::Bytes;
use futures_util::StreamExt;
use pin_project_lite::pin_project;
use warp::{
    hyper::{Body as HyperBody, Request as HyperRequest},
    Buf, Error as WarpError, Stream,
};

pub mod error;
pub mod utils;

use error::Error;

//
pin_project! {
    #[project = BodyProj]
    pub enum Body {
        Buf { inner: Box<dyn Buf + Send + 'static> },
        Bytes { inner: Bytes },
        Stream { #[pin] inner: Pin<Box<dyn Stream<Item = Result<Bytes, WarpError>> + Send + 'static>> },
        HyperBody { #[pin] inner: HyperBody }
    }
}

impl fmt::Display for Body {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Buf { inner: _ } => write!(f, "Buf"),
            Self::Bytes { inner: _ } => write!(f, "Bytes"),
            Self::Stream { inner: _ } => write!(f, "Stream"),
            Self::HyperBody { inner: _ } => write!(f, "HyperBody"),
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
    pub fn with_buf(buf: impl Buf + Send + 'static) -> Self {
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
            inner: utils::buf_stream_to_bytes_stream(stream).boxed(),
        }
    }

    pub fn with_hyper_body(hyper_body: HyperBody) -> Self {
        Self::HyperBody { inner: hyper_body }
    }

    pub fn require_to_bytes_async(&self) -> bool {
        matches!(
            self,
            Self::Stream { inner: _ } | Self::HyperBody { inner: _ }
        )
    }

    pub fn to_bytes(self) -> Bytes {
        match self {
            Self::Buf { inner } => utils::buf_to_bytes(inner),
            Self::Bytes { inner } => inner,
            Self::Stream { inner: _ } => panic!("Please call require_to_bytes_async first"),
            Self::HyperBody { inner: _ } => panic!("Please call require_to_bytes_async first"),
        }
    }

    pub async fn to_bytes_async(self) -> Result<Bytes, Error> {
        match self {
            Self::Buf { inner } => Ok(utils::buf_to_bytes(inner)),
            Self::Bytes { inner } => Ok(inner),
            Self::Stream { inner } => utils::bytes_stream_to_bytes(inner)
                .await
                .map_err(Into::into),
            Self::HyperBody { inner } => {
                utils::hyper_body_to_bytes(inner).await.map_err(Into::into)
            }
        }
    }
}

//

//
impl Stream for Body {
    type Item = Result<Bytes, Error>;

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
            BodyProj::Stream { inner } => inner.poll_next(cx).map_err(Into::into),
            BodyProj::HyperBody { inner } => inner.poll_next(cx).map_err(Into::into),
        }
    }
}

//
pub fn buf_request_to_body_request(
    req: HyperRequest<impl Buf + Send + 'static>,
) -> HyperRequest<Body> {
    let (parts, body) = req.into_parts();
    HyperRequest::from_parts(parts, Body::with_buf(body))
}

pub fn bytes_request_to_body_request(req: HyperRequest<Bytes>) -> HyperRequest<Body> {
    let (parts, body) = req.into_parts();
    HyperRequest::from_parts(parts, Body::with_bytes(body))
}

pub fn stream_request_to_body_request(
    req: HyperRequest<impl Stream<Item = Result<impl Buf + 'static, WarpError>> + Send + 'static>,
) -> HyperRequest<Body> {
    let (parts, body) = req.into_parts();
    HyperRequest::from_parts(parts, Body::with_stream(body))
}

pub fn hyper_body_request_to_body_request(req: HyperRequest<HyperBody>) -> HyperRequest<Body> {
    let (parts, body) = req.into_parts();
    HyperRequest::from_parts(parts, Body::with_hyper_body(body))
}

#[cfg(test)]
mod tests {
    use futures_util::{stream::BoxStream, StreamExt as _, TryStreamExt};

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

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_aggregate())
            .await
            .unwrap();
        let (_, body) = buf_request_to_body_request(req).into_parts();
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));
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

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_bytes())
            .await
            .unwrap();
        let (_, body) = bytes_request_to_body_request(req).into_parts();
        assert!(!body.require_to_bytes_async());
        assert_eq!(body.to_bytes(), Bytes::copy_from_slice(b"foo"));
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

        //
        let req = warp::test::request()
            .body("foo")
            .filter(&warp_filter_request::with_body_stream())
            .await
            .unwrap();
        let (_, body) = stream_request_to_body_request(req).into_parts();
        assert!(body.require_to_bytes_async());
        assert_eq!(
            body.to_bytes_async().await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
    }

    #[tokio::test]
    async fn test_with_hyper_body() {
        //
        let hyper_body = HyperBody::from("foo");
        let body = Body::with_hyper_body(hyper_body);
        assert!(body.require_to_bytes_async());
        assert_eq!(
            body.to_bytes_async().await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );

        //
        let hyper_body = HyperBody::from("foo");
        let mut body = Body::with_hyper_body(hyper_body);
        assert_eq!(
            body.next().await.unwrap().unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
        assert!(body.next().await.is_none());

        //
        let req = HyperRequest::new(HyperBody::from("foo"));
        let (_, body) = hyper_body_request_to_body_request(req).into_parts();
        assert!(body.require_to_bytes_async());
        assert_eq!(
            body.to_bytes_async().await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
    }

    pin_project! {
        pub struct BodyWrapper {
            #[pin]
            inner: BoxStream<'static, Result<Bytes, Box<dyn std::error::Error + Send + Sync + 'static>>>
        }
    }
    #[tokio::test]
    async fn test_wrapper() {
        //
        let buf = warp::test::request()
            .body("foo")
            .filter(&warp::body::aggregate())
            .await
            .unwrap();
        let body = Body::with_buf(buf);
        let _ = BodyWrapper {
            inner: body.err_into().boxed(),
        };

        //
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();
        let body = Body::with_stream(stream);
        let _ = BodyWrapper {
            inner: body.err_into().boxed(),
        };
    }
}
