use crates_io_session::COOKIE_NAME;
use http::header;
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

const DESCRIPTION: &str = r#"
__Experimental API documentation for the [crates.io](https://crates.io/)
package registry.__

This document describes the API used by the crates.io website, cargo
client, and other third-party tools to interact with the crates.io
registry.

Before using this API, please read the
[crates.io data access policy](https://crates.io/data-access) and ensure
that your use of the API complies with the policy.

__The API is under active development and may change at any time__,
though we will try to avoid breaking changes where possible.

Some parts of the API follow the "Registry Web API" spec documented
at <https://doc.rust-lang.org/cargo/reference/registry-web-api.html>
and can be considered stable.

Most parts of the API do not require authentication. The endpoints
that do require authentication are marked as such in the documentation,
with some requiring cookie authentication (usable only by the web UI)
and others requiring API token authentication (usable by cargo and
other clients).
"#;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "crates.io",
        description = DESCRIPTION,
        terms_of_service = "https://crates.io/policies",
        contact(name = "the crates.io team", email = "help@crates.io"),
        license(name = "MIT OR Apache-2.0", url = "https://github.com/rust-lang/crates.io/blob/main/README.md#%EF%B8%8F-license"),
        version = "0.0.0",
    ),
    modifiers(&SecurityAddon),
    servers(
        (url = "https://crates.io"),
        (url = "https://staging.crates.io"),
    ),
)]
pub struct BaseOpenApi;

impl BaseOpenApi {
    pub fn router<S>() -> OpenApiRouter<S>
    where
        S: Send + Sync + Clone + 'static,
    {
        OpenApiRouter::with_openapi(Self::openapi())
    }
}

struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.get_or_insert_default();

        let description = "The session cookie is used by the web UI to authenticate users.";
        let cookie = ApiKey::Cookie(ApiKeyValue::with_description(COOKIE_NAME, description));
        components.add_security_scheme("cookie", SecurityScheme::ApiKey(cookie));

        let name = header::AUTHORIZATION.as_str();
        let description =
            "The API token is used to authenticate requests from cargo and other clients.";
        let api_token = ApiKey::Header(ApiKeyValue::with_description(name, description));
        components.add_security_scheme("api_token", SecurityScheme::ApiKey(api_token));

        let description = "Temporary access tokens are used by the \"Trusted Publishing\" flow.";
        let trustpub_token = Http::builder()
            .scheme(HttpAuthScheme::Bearer)
            .description(Some(description))
            .build();

        components.add_security_scheme("trustpub_token", SecurityScheme::Http(trustpub_token));
    }
}

#[cfg(test)]
mod tests {
    use crate::tests::util::{RequestHelper, TestApp};
    use insta::{assert_json_snapshot, assert_snapshot};

    #[tokio::test(flavor = "multi_thread")]
    async fn test_openapi_snapshot() {
        let (_app, anon) = TestApp::init().empty().await;

        let response = anon.get::<()>("/api/openapi.json").await;
        assert_snapshot!(response.status(), @"200 OK");
        assert_json_snapshot!(response.json());
    }
}
