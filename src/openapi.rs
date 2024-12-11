use utoipa::OpenApi;
use utoipa_axum::router::OpenApiRouter;

#[derive(OpenApi)]
#[openapi(info(
    title = "crates.io",
    description = "API documentation for the [crates.io](https://crates.io/) package registry",
    terms_of_service = "https://crates.io/policies",
    contact(name = "the crates.io team", email = "help@crates.io"),
    license(),
    version = "0.0.0",
))]
pub struct BaseOpenApi;

impl BaseOpenApi {
    pub fn router<S>() -> OpenApiRouter<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        OpenApiRouter::with_openapi(Self::openapi())
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::util::{RequestHelper, TestApp};
    use http::StatusCode;
    use insta::assert_json_snapshot;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_openapi_snapshot() {
        let (_app, anon) = TestApp::init().empty().await;

        let response = anon.get::<()>("/api/openapi.json").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!(response.json());
    }
}
