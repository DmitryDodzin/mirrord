use std::sync::Arc;

use http::{HeaderName, HeaderValue, Request};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct CustomHeaders<S> {
    inner: S,
    headers: Arc<Vec<(HeaderName, HeaderValue)>>,
}

impl<S, ReqBody> Service<Request<ReqBody>> for CustomHeaders<S>
where
    S: Service<Request<ReqBody>>,
{
    type Error = S::Error;
    type Future = S::Future;
    type Response = S::Response;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        req.headers_mut().extend(self.headers.iter().cloned());
        self.inner.call(req)
    }
}

#[derive(Debug)]
pub struct CustomHeadersLayer {
    headers: Arc<Vec<(HeaderName, HeaderValue)>>,
}

impl<A> FromIterator<A> for CustomHeadersLayer
where
    A: Into<(HeaderName, HeaderValue)>,
{
    fn from_iter<T>(items: T) -> Self
    where
        T: IntoIterator<Item = A>,
    {
        CustomHeadersLayer {
            headers: Arc::new(items.into_iter().map(|value| value.into()).collect()),
        }
    }
}

impl<S> Layer<S> for CustomHeadersLayer {
    type Service = CustomHeaders<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CustomHeaders {
            inner,
            headers: self.headers.clone(),
        }
    }
}

pub struct SessionId(HeaderValue);

impl SessionId {
    pub fn new(session_id: &str) -> Result<Self, http::Error> {
        HeaderValue::from_str(session_id)
            .map(SessionId)
            .map_err(http::Error::from)
    }
}

impl IntoIterator for SessionId {
    type Item = (HeaderName, HeaderValue);

    type IntoIter = std::iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once((HeaderName::from_static("x-session-id"), self.0))
    }
}

pub struct Version;

impl IntoIterator for Version {
    type Item = (HeaderName, HeaderValue);

    type IntoIter = std::iter::Once<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        std::iter::once((
            HeaderName::from_static("x-layer-version"),
            HeaderValue::from_static(env!("CARGO_PKG_VERSION")),
        ))
    }
}
