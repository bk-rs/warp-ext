use bytes::{Bytes, BytesMut};
use futures_util::{StreamExt as _, TryStreamExt as _};
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
    mut stream: impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
) -> Result<Bytes, WarpError> {
    let mut bytes_mut = BytesMut::new();
    while let Some(buf) = stream.next().await {
        let buf = buf?;
        bytes_mut.extend_from_slice(&buf_to_bytes(buf)[..]);
    }
    Ok(bytes_mut.freeze())
}

pub fn buf_stream_to_bytes_stream(
    stream: impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static,
) -> impl Stream<Item = Result<Bytes, WarpError>> + Send + 'static {
    stream.map_ok(|buf| buf_to_bytes(buf))
}

pub async fn bytes_stream_to_bytes(
    mut stream: impl Stream<Item = Result<Bytes, WarpError>> + Send + 'static + Unpin,
) -> Result<Bytes, WarpError> {
    let mut bytes_mut = BytesMut::new();
    while let Some(bytes) = stream.next().await {
        let bytes = bytes?;
        bytes_mut.extend_from_slice(&bytes[..]);
    }
    Ok(bytes_mut.freeze())
}
