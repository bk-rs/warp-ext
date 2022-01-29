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
}
