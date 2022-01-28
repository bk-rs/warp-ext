use warp::{
    hyper::{
        http::{HeaderMap, Method, Uri},
        Request as HyperRequest,
    },
    path::FullPath,
    Rejection,
};

use crate::error::Error;

pub(crate) fn make_request_without_body<T1, T2>(
    method: Method,
    path: FullPath,
    query: Option<String>,
    headers: HeaderMap,
    extensions: (Option<T1>, Option<T2>),
) -> Result<HyperRequest<()>, Rejection>
where
    T1: Clone + Send + Sync + 'static,
    T2: Clone + Send + Sync + 'static,
{
    let uri = if let Some(query) = query {
        Uri::builder()
            .path_and_query(format!("{}?{}", path.as_str(), query))
            .build()
            .map_err(|err| Rejection::from(Error::from(err)))?
    } else {
        Uri::builder()
            .path_and_query(path.as_str())
            .build()
            .map_err(|err| Rejection::from(Error::from(err)))?
    };

    let (mut parts, _) = HyperRequest::new(()).into_parts();
    parts.method = method;
    parts.uri = uri;
    parts.headers = headers;

    if let Some(extension) = extensions.0 {
        parts.extensions.insert(extension);
    }
    if let Some(extension) = extensions.1 {
        parts.extensions.insert(extension);
    }

    Ok(HyperRequest::from_parts(parts, ()))
}
