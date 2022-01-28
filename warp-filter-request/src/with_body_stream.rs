use warp::{
    hyper::{
        http::{HeaderMap, Method},
        Request as HyperRequest,
    },
    path::FullPath,
    Buf, Error as WarpError, Filter, Rejection, Stream,
};

use crate::common::make_request_without_body;

//
pub fn with_body_stream() -> impl Filter<
    Extract = (HyperRequest<impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static>,),
    Error = Rejection,
> + Copy {
    warp::any()
        .and(warp::method())
        .and(warp::filters::path::full())
        .and(
            warp::filters::query::raw()
                .map(Some)
                .or(warp::any().map(|| None))
                .unify(),
        )
        .and(warp::header::headers_cloned())
        .and(warp::body::stream())
        .and_then(|method, path, query, headers, body| async move {
            make_request(
                method,
                path,
                query,
                headers,
                body,
                (Option::<()>::None, Option::<()>::None),
            )
        })
}

pub fn with_body_stream_and_one_extension<T1>() -> impl Filter<
    Extract = (HyperRequest<impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static>,),
    Error = Rejection,
> + Copy
where
    T1: Clone + Send + Sync + 'static,
{
    warp::any()
        .and(warp::method())
        .and(warp::filters::path::full())
        .and(
            warp::filters::query::raw()
                .map(Some)
                .or(warp::any().map(|| None))
                .unify(),
        )
        .and(warp::header::headers_cloned())
        .and(warp::body::stream())
        .and(warp::ext::optional::<T1>())
        .and_then(|method, path, query, headers, body, ext1| async move {
            make_request(
                method,
                path,
                query,
                headers,
                body,
                (ext1, Option::<()>::None),
            )
        })
}

pub fn with_body_stream_and_two_extensions<T1, T2>() -> impl Filter<
    Extract = (HyperRequest<impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static>,),
    Error = Rejection,
> + Copy
where
    T1: Clone + Send + Sync + 'static,
    T2: Clone + Send + Sync + 'static,
{
    warp::any()
        .and(warp::method())
        .and(warp::filters::path::full())
        .and(
            warp::filters::query::raw()
                .map(Some)
                .or(warp::any().map(|| None))
                .unify(),
        )
        .and(warp::header::headers_cloned())
        .and(warp::body::stream())
        .and(warp::ext::optional::<T1>())
        .and(warp::ext::optional::<T2>())
        .and_then(
            |method, path, query, headers, body, ext1, ext2| async move {
                make_request(method, path, query, headers, body, (ext1, ext2))
            },
        )
}

fn make_request<T1, T2>(
    method: Method,
    path: FullPath,
    query: Option<String>,
    headers: HeaderMap,
    body: impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static,
    extensions: (Option<T1>, Option<T2>),
) -> Result<HyperRequest<impl Stream<Item = Result<impl Buf, WarpError>> + Send + 'static>, Rejection>
where
    T1: Clone + Send + Sync + 'static,
    T2: Clone + Send + Sync + 'static,
{
    let (parts, _) =
        make_request_without_body(method, path, query, headers, extensions)?.into_parts();

    Ok(HyperRequest::from_parts(parts, body))
}
