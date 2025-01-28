//! Serve the Ember.js frontend HTML
//!
//! Paths intended for the inner `api_handler` are passed along to the remaining middleware layers
//! as normal. Requests not intended for the backend will be served HTML to boot the Ember.js
//! frontend.
//!
//! For now, there is an additional check to see if the `Accept` header contains "html". This is
//! likely to be removed in the future.

use std::borrow::Cow;
use std::path::Path;
use std::sync::{Arc, OnceLock};

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures_util::future::{BoxFuture, Shared};
use futures_util::FutureExt;
use http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use minijinja::{context, Environment};

use crate::app::AppState;

const OG_IMAGE_FALLBACK_URL: &str = "https://crates.io/assets/og-image.png";
const INDEX_TEMPLATE_NAME: &str = "index_html";
const PATH_PREFIX_CRATES: &str = "/crates/";

/// The [`Shared`] allows for multiple tasks to wait on a single future, [`BoxFuture`] allows
/// us to name the type in the declaration of static variables, and the [`Arc`] ensures
/// the [`minijinja::Environment`] doensn't get cloned each request.
type TemplateEnvFut = Shared<BoxFuture<'static, Arc<minijinja::Environment<'static>>>>;
type TemplateCache = moka::future::Cache<Cow<'static, str>, String>;

/// Initialize [`minijinja::Environment`] given the path to the index.html file. This should
/// only be done once as it will load said file from persistent storage.
async fn init_template_env(
    index_html_template_path: impl AsRef<Path>,
) -> Arc<minijinja::Environment<'static>> {
    let template_j2 = tokio::fs::read_to_string(index_html_template_path.as_ref())
        .await
        .expect("Error loading index.html template. Is the frontend package built yet?");

    let mut env = Environment::empty();
    env.add_template_owned(INDEX_TEMPLATE_NAME, template_j2)
        .expect("Error loading template");
    Arc::new(env)
}

/// Initialize the [`moka::future::Cache`] used to cache the rendered HTML.
fn init_html_cache(max_capacity: u64) -> TemplateCache {
    moka::future::CacheBuilder::new(max_capacity)
        .name("rendered_index_html")
        .build()
}

pub async fn serve_html(state: AppState, request: Request, next: Next) -> Response {
    static TEMPLATE_ENV: OnceLock<TemplateEnvFut> = OnceLock::new();
    static RENDERED_HTML_CACHE: OnceLock<TemplateCache> = OnceLock::new();

    let path = &request.uri().path();
    // The "/git/" prefix is only used in development (when within a docker container)
    if path.starts_with("/api/") || path.starts_with("/git/") {
        next.run(request).await
    } else if request
        .headers()
        .get_all(header::ACCEPT)
        .iter()
        .any(|val| val.to_str().unwrap_or_default().contains("html"))
    {
        if !matches!(*request.method(), Method::HEAD | Method::GET) {
            let headers =
                HeaderMap::from_iter([(header::ALLOW, HeaderValue::from_static("GET,HEAD"))]);
            return (StatusCode::METHOD_NOT_ALLOWED, headers).into_response();
        }

        // Come up with an Open Graph image URL. In case a crate page is requested,
        // we use the crate's name and the OG image base URL from config to
        // generate one, otherwise we use the fallback image.
        let og_image_url = 'og: {
            if let Some(suffix) = path.strip_prefix(PATH_PREFIX_CRATES) {
                let len = suffix.find('/').unwrap_or(suffix.len());
                let krate = &suffix[..len];

                // `state.config.og_image_base_url` will always be `Some` as that's required
                // if `state.config.index_html_template_path` is `Some`, and otherwise this
                // middleware won't be executed; see `crate::middleware::apply_axum_middleware`.
                if let Ok(og_img_url) = state.config.og_image_base_url.as_ref().unwrap().join(krate)
                {
                    break 'og Cow::from(og_img_url.to_string());
                }
            }
            OG_IMAGE_FALLBACK_URL.into()
        };

        // Fetch the HTML from cache given `og_image_url` as key or render it
        let html = RENDERED_HTML_CACHE
            .get_or_init(|| init_html_cache(state.config.html_render_cache_max_capacity))
            .get_with_by_ref(&og_image_url, async {
                // `OnceLock::get_or_init` blocks as long as its intializer is running in another thread.
                // Note that this won't take long, as the constructed Futures are not awaited
                // during initialization.
                let template_env = TEMPLATE_ENV.get_or_init(|| {
                    // At this point we can safely assume `state.config.index_html_template_path` is `Some`,
                    // as this middleware won't be executed otherwise; see `crate::middleware::apply_axum_middleware`.
                    init_template_env(state.config.index_html_template_path.clone().unwrap())
                        .boxed()
                        .shared()
                });

                // Render the HTML given the OG image URL
                let env = template_env.clone().await;
                let html = env
                    .get_template(INDEX_TEMPLATE_NAME)
                    .unwrap()
                    .render(context! { og_image_url})
                    .expect("Error rendering index");

                html
            })
            .await;

        // Serve static Ember page to bootstrap the frontend
        Response::builder()
            .header(header::CONTENT_TYPE, "text/html")
            .header(header::CONTENT_LENGTH, html.len())
            .body(axum::body::Body::new(html))
            .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
    } else {
        // Return a 404 to crawlers that don't send `Accept: text/hml`.
        // This is to preserve legacy behavior and will likely change.
        // Most of these crawlers probably won't execute our frontend JS anyway, but
        // it would be nice to bootstrap the app for crawlers that do execute JS.
        StatusCode::NOT_FOUND.into_response()
    }
}
