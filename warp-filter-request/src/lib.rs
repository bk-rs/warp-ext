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
    use futures_util::StreamExt;
    use warp::{
        http::Error as WarpHttpError,
        hyper::{http::Method, Body, Client, Request, Response},
        Buf, Error as WarpError, Filter as _, Stream,
    };

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
                assert_eq!(body.chunk(), b"aggregate_1");

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
                let (_, _) = req.into_parts();
                // TODO

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-aggregate_2", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_aggregate_2 = warp::path("aggregate_2")
                .and(with_body_aggregate_and_one_extension::<()>())
                .and_then(aggregate_2);

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
            async fn stream_1(
                req: Request<
                    impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static + Unpin,
                >,
            ) -> Result<Result<Response<Body>, WarpHttpError>, Infallible> {
                let (_, mut body) = req.into_parts();
                let mut body_bytes = vec![];
                while let Some(buf) = body.next().await {
                    let buf = buf.unwrap();
                    body_bytes.extend_from_slice(buf.chunk());
                }
                assert_eq!(body_bytes, b"stream_1");

                let mut res = Response::new(Body::empty());
                res.headers_mut().insert("x-stream_1", 1.into());
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(res))
            }
            let filter_stream_1 = warp::path("stream_1")
                .and(with_body_stream())
                .and_then(stream_1);

            //
            warp::serve(
                filter_aggregate_1
                    .or(filter_aggregate_2)
                    .or(filter_bytes_1)
                    .or(filter_stream_1),
            )
            .run(listen_addr)
            .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

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
                    .uri(format!("http://{}{}", listen_addr, "/bytes_1"))
                    .body(Body::from("bytes_1"))
                    .unwrap(),
                "x-bytes_1",
            ),
            (
                Request::builder()
                    .uri(format!("http://{}{}", listen_addr, "/stream_1"))
                    .body(Body::from("stream_1"))
                    .unwrap(),
                "x-stream_1",
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
