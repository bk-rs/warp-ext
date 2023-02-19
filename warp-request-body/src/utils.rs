use bytes::{Bytes, BytesMut};
use futures_util::{StreamExt as _, TryStreamExt as _};
pub use hyper_body_to_bytes::{hyper_body_to_bytes, hyper_body_to_bytes_with_max_length};
use warp::{Buf, Error as WarpError, Stream};

pub fn buf_to_bytes(mut buf: impl Buf) -> Bytes {
    let mut bytes_mut = BytesMut::new();
    while buf.has_remaining() {
        bytes_mut.extend_from_slice(buf.chunk());
        let cnt = buf.chunk().len();
        buf.advance(cnt);
    }
    bytes_mut.freeze()
}

pub async fn buf_stream_to_bytes(
    mut stream: impl Stream<Item = Result<impl Buf, WarpError>> + Unpin,
) -> Result<Bytes, WarpError> {
    let mut bytes_mut = BytesMut::new();
    while let Some(buf) = stream.next().await {
        let buf = buf?;
        bytes_mut.extend_from_slice(&buf_to_bytes(buf)[..]);
    }
    Ok(bytes_mut.freeze())
}

pub fn buf_stream_to_bytes_stream(
    stream: impl Stream<Item = Result<impl Buf, WarpError>>,
) -> impl Stream<Item = Result<Bytes, WarpError>> {
    stream.map_ok(|buf| buf_to_bytes(buf))
}

pub async fn bytes_stream_to_bytes(
    mut stream: impl Stream<Item = Result<Bytes, WarpError>> + Unpin,
) -> Result<Bytes, WarpError> {
    let mut bytes_mut = BytesMut::new();
    while let Some(bytes) = stream.next().await {
        let bytes = bytes?;
        bytes_mut.extend_from_slice(&bytes[..]);
    }
    Ok(bytes_mut.freeze())
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures_util::stream::BoxStream;

    #[tokio::test]
    async fn test_buf_to_bytes() {
        let buf = warp::test::request()
            .body("foo")
            .filter(&warp::body::aggregate())
            .await
            .unwrap();
        assert_eq!(buf_to_bytes(buf), Bytes::copy_from_slice(b"foo"));
    }

    #[tokio::test]
    async fn test_buf_stream_to_bytes() {
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();
        assert_eq!(
            buf_stream_to_bytes(stream).await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
    }

    #[tokio::test]
    async fn test_buf_stream_to_bytes_stream() {
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();
        let _: BoxStream<'static, Result<Bytes, WarpError>> =
            buf_stream_to_bytes_stream(stream).boxed();
    }

    #[tokio::test]
    async fn test_bytes_stream_to_bytes() {
        let stream = warp::test::request()
            .body("foo")
            .filter(&warp::body::stream())
            .await
            .unwrap();

        let stream = buf_stream_to_bytes_stream(stream);
        assert_eq!(
            bytes_stream_to_bytes(stream).await.unwrap(),
            Bytes::copy_from_slice(b"foo")
        );
    }
}
