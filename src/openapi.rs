use axum::Extension;
use axum::body::Bytes;
use axum::extract::Query;
use axum::response::IntoResponse;
use axum_extra::TypedHeader;
use axum_extra::headers::ContentType;
use crates_io_session::COOKIE_NAME;
use http::header;
use serde::Deserialize;
use std::sync::{Arc, OnceLock};
use utoipa::openapi::OpenApi as OpenApiDoc;
use utoipa::openapi::path::{Operation, PathItem};
use utoipa::openapi::security::{ApiKey, ApiKeyValue, Http, HttpAuthScheme, SecurityScheme};
use utoipa::{Modify, OpenApi};
use utoipa_axum::router::OpenApiRouter;

const X_INTERNAL: &str = "x-internal";

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

#[derive(Deserialize)]
pub struct Params {
    #[serde(default)]
    internal: Option<String>,
}

pub async fn handler(
    Extension(doc): Extension<Arc<OpenApiDoc>>,
    Query(params): Query<Params>,
) -> impl IntoResponse {
    static PUBLIC_BYTES: OnceLock<Bytes> = OnceLock::new();
    static FULL_BYTES: OnceLock<Bytes> = OnceLock::new();

    let cache = match params.internal {
        None => &PUBLIC_BYTES,
        Some(_) => &FULL_BYTES,
    };

    let bytes = cache.get_or_init(|| {
        let mut doc = (*doc).clone();
        apply_visibility(&mut doc, params.internal.is_some());
        Bytes::from(serde_json::to_vec(&doc).expect("OpenAPI serialization failed"))
    });

    (TypedHeader(ContentType::json()), bytes.clone())
}

/// Mutate `openapi` in place: drop operations marked `x-internal: true` when
/// `include_internal` is false, and strip the marker from any remaining
/// operations regardless.
fn apply_visibility(openapi: &mut OpenApiDoc, include_internal: bool) {
    openapi.paths.paths.retain(|_, item| {
        prune_path_item(item, include_internal);
        !path_item_is_empty(item)
    });
}

fn prune_path_item(item: &mut PathItem, include_internal: bool) {
    for slot in [
        &mut item.get,
        &mut item.put,
        &mut item.post,
        &mut item.delete,
        &mut item.options,
        &mut item.head,
        &mut item.patch,
        &mut item.trace,
    ] {
        let Some(op) = slot.as_mut() else { continue };
        if !include_internal && is_internal(op) {
            *slot = None;
        } else if let Some(ext) = op.extensions.as_mut() {
            ext.remove(X_INTERNAL);
        }
    }
}

fn is_internal(op: &Operation) -> bool {
    op.extensions
        .as_ref()
        .and_then(|ext| ext.get(X_INTERNAL))
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn path_item_is_empty(item: &PathItem) -> bool {
    item.get.is_none()
        && item.put.is_none()
        && item.post.is_none()
        && item.delete.is_none()
        && item.options.is_none()
        && item.head.is_none()
        && item.patch.is_none()
        && item.trace.is_none()
}
