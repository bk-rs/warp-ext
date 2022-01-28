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

#[cfg(test)]
mod tests {
    use super::*;

    use core::convert::Infallible;
    use std::{error, net::SocketAddr};

    use bytes::Bytes;
    use futures_util::StreamExt as _;
    use warp::{
        http::Error as WarpHttpError,
        hyper::{http::Method, Body, Client, Request, Response},
        Buf, Error as WarpError, Filter as _, Stream,
    };

    #[derive(Debug, Clone, PartialEq)]
    struct Ext1(usize);

    #[derive(Debug, Clone, PartialEq)]
    struct Ext2(isize);

    #[tokio::test]
    async fn test_and_extension() {
        //
        let req = warp::test::request()
            .extension(Ext1(1))
            .filter(&with_body_aggregate_and_one_extension::<Ext1>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));

        let req = warp::test::request()
            .extension(Ext1(1))
            .extension(Ext2(-1))
            .filter(&with_body_aggregate_and_two_extensions::<Ext1, Ext2>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));
        assert_eq!(req.extensions().get::<Ext2>().unwrap(), &Ext2(-1));

        //
        let req = warp::test::request()
            .extension(Ext1(1))
            .filter(&with_body_bytes_and_one_extension::<Ext1>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));

        let req = warp::test::request()
            .extension(Ext1(1))
            .extension(Ext2(-1))
            .filter(&with_body_bytes_and_two_extensions::<Ext1, Ext2>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));
        assert_eq!(req.extensions().get::<Ext2>().unwrap(), &Ext2(-1));

        //
        let req = warp::test::request()
            .extension(Ext1(1))
            .filter(&with_body_stream_and_one_extension::<Ext1>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));

        let req = warp::test::request()
            .extension(Ext1(1))
            .extension(Ext2(-1))
            .filter(&with_body_stream_and_two_extensions::<Ext1, Ext2>())
            .await
            .unwrap();
        assert_eq!(req.extensions().get::<Ext1>().unwrap(), &Ext1(1));
        assert_eq!(req.extensions().get::<Ext2>().unwrap(), &Ext2(-1));
    }

    //
    fn buf_to_bytes(mut buf: impl Buf) -> Vec<u8> {
        let mut bytes = vec![];
        while buf.has_remaining() {
            bytes.extend_from_slice(buf.chunk());
            let cnt = buf.chunk().len();
            buf.advance(cnt);
        }
        bytes
    }

    async fn stream_to_bytes(
        mut stream: impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
    ) -> Vec<u8> {
        let mut bytes = vec![];
        while let Some(buf) = stream.next().await {
            let buf = buf.unwrap();
            bytes.extend_from_slice(buf.chunk());
        }
        bytes
    }

    #[tokio::test]
    async fn integration_test() -> Result<(), Box<dyn error::Error>> {
        let listen_addr =
            SocketAddr::from(([127, 0, 0, 1], portpicker::pick_unused_port().unwrap()));
        println!("listen_addr {:?}", listen_addr);

        let server_task = tokio::task::spawn(async move {
            //
            async fn aggregate_1(
                req: Request<impl Buf>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (parts, body) = req.into_parts();
                println!("{:?}", parts);
                assert_eq!(&parts.method, Method::POST);
                assert_eq!(parts.uri.path(), "/aggregate_1");
                assert_eq!(parts.uri.query(), Some("foo=1"));
                assert_eq!(parts.headers.get("x-bar").unwrap(), "1");
                assert_eq!(buf_to_bytes(body), b"aggregate_1");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-aggregate_1", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_aggregate_1 = warp::path("aggregate_1")
                .and(with_body_aggregate())
                .and_then(aggregate_1);

            //
            async fn aggregate_2(
                req: Request<impl Buf>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(buf_to_bytes(body), b"aggregate_2");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-aggregate_2", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_aggregate_2 = warp::path("aggregate_2")
                .and(with_body_aggregate_and_one_extension::<()>())
                .and_then(aggregate_2);

            //
            async fn aggregate_3(
                req: Request<impl Buf>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(buf_to_bytes(body), b"aggregate_3");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-aggregate_3", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_aggregate_3 = warp::path("aggregate_3")
                .and(with_body_aggregate_and_two_extensions::<(), ()>())
                .and_then(aggregate_3);

            //
            async fn bytes_1(
                req: Request<Bytes>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(&body[..], b"bytes_1");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-bytes_1", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_bytes_1 = warp::path("bytes_1")
                .and(with_body_bytes())
                .and_then(bytes_1);

            //
            async fn bytes_2(
                req: Request<Bytes>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(&body[..], b"bytes_2");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-bytes_2", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_bytes_2 = warp::path("bytes_2")
                .and(with_body_bytes_and_one_extension::<()>())
                .and_then(bytes_2);

            //
            async fn bytes_3(
                req: Request<Bytes>,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(&body[..], b"bytes_3");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-bytes_3", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_bytes_3 = warp::path("bytes_3")
                .and(with_body_bytes_and_two_extensions::<(), ()>())
                .and_then(bytes_3);

            //
            async fn stream_1(
                req: Request<
                    impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
                >,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(stream_to_bytes(body).await, b"stream_1");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-stream_1", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_stream_1 = warp::path("stream_1")
                .and(with_body_stream())
                .and_then(stream_1);

            //
            async fn stream_2(
                req: Request<
                    impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
                >,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(stream_to_bytes(body).await, b"stream_2");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-stream_2", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_stream_2 = warp::path("stream_2")
                .and(with_body_stream_and_one_extension::<()>())
                .and_then(stream_2);

            //
            async fn stream_3(
                req: Request<
                    impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
                >,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, body) = req.into_parts();
                assert_eq!(stream_to_bytes(body).await, b"stream_3");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-stream_3", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_stream_3 = warp::path("stream_3")
                .and(with_body_stream_and_two_extensions::<(), ()>())
                .and_then(stream_3);

            //
            warp::serve(
                filter_aggregate_1
                    .or(filter_aggregate_2)
                    .or(filter_aggregate_3)
                    .or(filter_bytes_1)
                    .or(filter_bytes_2)
                    .or(filter_bytes_3)
                    .or(filter_stream_1)
                    .or(filter_stream_2)
                    .or(filter_stream_3),
            )
            .run(listen_addr)
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        //
        let client = Client::new();

        //
        for (req, res_header_key) in vec![
            (
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("http://{}{}", listen_addr, "/aggregate_1?foo=1"))
                    .header("x-bar", "1")
                    .body(Body::from("aggregate_1"))
                    .unwrap(),
                "x-aggregate_1",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/aggregate_2"))
                    .body(Body::from("aggregate_2"))
                    .unwrap(),
                "x-aggregate_2",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/aggregate_3"))
                    .body(Body::from("aggregate_3"))
                    .unwrap(),
                "x-aggregate_3",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/bytes_1"))
                    .body(Body::from("bytes_1"))
                    .unwrap(),
                "x-bytes_1",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/bytes_2"))
                    .body(Body::from("bytes_2"))
                    .unwrap(),
                "x-bytes_2",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/bytes_3"))
                    .body(Body::from("bytes_3"))
                    .unwrap(),
                "x-bytes_3",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/stream_1"))
                    .body(Body::from("stream_1"))
                    .unwrap(),
                "x-stream_1",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/stream_2"))
                    .body(Body::from("stream_2"))
                    .unwrap(),
                "x-stream_2",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/stream_3"))
                    .body(Body::from("stream_3"))
                    .unwrap(),
                "x-stream_3",
            ),
        ] {
            let res = client.request(req).await?;
            assert!(res.status().is_success());
            assert_eq!(res.headers().get(res_header_key).unwrap(), "1");
        }

        //
        server_task.abort();
        assert!(server_task.await.unwrap_err().is_cancelled());

        Ok(())
    }
}
