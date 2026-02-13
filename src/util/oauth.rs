use std::future::Future;
use std::pin::Pin;

/// Bridges `reqwest::Client` with `oauth2::AsyncHttpClient` so that
/// oauth2 can be used without pulling in its default reqwest feature.
pub struct ReqwestClient(pub reqwest::Client);

impl<'c> oauth2::AsyncHttpClient<'c> for ReqwestClient {
    type Error = oauth2::HttpClientError<reqwest::Error>;

    type Future =
        Pin<Box<dyn Future<Output = Result<oauth2::HttpResponse, Self::Error>> + Send + Sync + 'c>>;

    fn call(&'c self, request: oauth2::HttpRequest) -> Self::Future {
        Box::pin(async move {
            let response = self
                .0
                .execute(request.try_into().map_err(Box::new)?)
                .await
                .map_err(Box::new)?;

            let mut builder = http::Response::builder()
                .status(response.status())
                .version(response.version());

            for (name, value) in response.headers().iter() {
                builder = builder.header(name, value);
            }

            builder
                .body(response.bytes().await.map_err(Box::new)?.to_vec())
                .map_err(oauth2::HttpClientError::Http)
        })
    }
}
