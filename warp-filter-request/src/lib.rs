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

    use warp::{
        http::Error as WarpHttpError,
        hyper::{http::Method, Body, Client, Request, Response},
        Buf, Filter as _,
    };

    #[derive(Debug, Clone, PartialEq)]
    struct Extension1(usize);
    #[derive(Debug, Clone, PartialEq)]
    struct Extension2(isize);

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
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(Response::new(
                    Body::empty(),
                )))
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
                Result::<Result<Response<Body>, WarpHttpError>, Infallible>::Ok(Ok(Response::new(
                    Body::empty(),
                )))
            }
            let filter_aggregate_2 = warp::path("aggregate_2")
                .and(with_body_aggregate_and_one_extension::<()>())
                .and_then(aggregate_2);

            //
            warp::serve(filter_aggregate_1.or(filter_aggregate_2))
                .run(listen_addr)
                .await
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        //
        let client = Client::new();

        //
        for req in vec![
            Request::builder()
                .method(Method::POST)
                .uri(format!("http://{}{}", listen_addr, "/aggregate_1?foo=1"))
                .header("x-bar", "1")
                .body(Body::from("aggregate_1"))
                .unwrap(),
            Request::builder()
                .uri(format!("http://{}{}", listen_addr, "/aggregate_2"))
                .body(Body::from("aggregate_2"))
                .unwrap(),
        ] {
            let res = client.request(req).await?;
            assert!(res.status().is_success());
        }

        //
        server_task.abort();
        assert!(server_task.await.unwrap_err().is_cancelled());

        Ok(())
    }
}
