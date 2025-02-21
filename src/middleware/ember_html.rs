//! Serve the Ember.js frontend HTML
//!
//! Paths intended for the inner `api_handler` are passed along to the remaining middleware layers
//! as normal. Requests not intended for the backend will be served HTML to boot the Ember.js
//! frontend.
//!
//! For now, there is an additional check to see if the `Accept` header contains "html". This is
//! likely to be removed in the future.

use std::borrow::Cow;
use std::ops::Not;
use std::sync::{Arc, LazyLock, OnceLock};

use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use futures_util::FutureExt;
use futures_util::future::{BoxFuture, Shared};
use http::{HeaderMap, HeaderValue, Method, StatusCode, header};
use url::Url;

use crate::app::AppState;

const OG_IMAGE_FALLBACK_URL: &str = "https://crates.io/assets/og-image.png";
const INDEX_TEMPLATE_NAME: &str = "index_html";
const PATH_PREFIX_CRATES: &str = "/crates/";

/// The [`Shared`] allows for multiple tasks to wait on a single future, [`BoxFuture`] allows
/// us to name the type in the declaration of static variables, and the [`Arc`] ensures
/// the [`minijinja::Environment`] doesn't get cloned each request.
type TemplateEnvFut = Shared<BoxFuture<'static, Arc<minijinja::Environment<'static>>>>;
type TemplateCache = moka::future::Cache<Cow<'static, str>, String>;

/// Initialize [`minijinja::Environment`] given the index.html file at `dist/index.html`.
/// This should only be done once as it will load said file from persistent storage.
async fn init_template_env() -> Arc<minijinja::Environment<'static>> {
    let template_j2 = tokio::fs::read_to_string("dist/index.html")
        .await
        .expect("Error loading dist/index.html template. Is the frontend package built yet?");

    let mut env = minijinja::Environment::empty();
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
    static TEMPLATE_ENV: LazyLock<TemplateEnvFut> =
        LazyLock::new(|| init_template_env().boxed().shared());
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

        // `state.config.og_image_base_url` will always be `Some` as that's required
        // if `state.config.serve_html` is `true`, and otherwise this
        // middleware won't be executed; see `crate::middleware::apply_axum_middleware`.
        let og_image_url = generate_og_image_url(path, state.config.og_image_base_url.as_ref())
            .map(|url| Cow::Owned(url.to_string()))
            .unwrap_or(Cow::Borrowed(OG_IMAGE_FALLBACK_URL));

        // Fetch the HTML from cache given `og_image_url` as key or render it
        let html_cache = RENDERED_HTML_CACHE
            .get_or_init(|| init_html_cache(state.config.html_render_cache_max_capacity));

        let render_result = html_cache
            .entry_by_ref(&og_image_url)
            .or_try_insert_with::<_, minijinja::Error>(async {
                // `LazyLock::deref` blocks as long as its initializer is running in another thread.
                // Note that this won't take long, as the constructed Futures are not awaited
                // during initialization.
                let template_env = &*TEMPLATE_ENV;

                // Render the HTML given the OG image URL
                let env = template_env.clone().await;
                let html = env
                    .get_template(INDEX_TEMPLATE_NAME)?
                    .render(minijinja::context! { og_image_url})?;

                Ok(html)
            })
            .await;

        match render_result {
            Ok(entry) => {
                // Serve static Ember page to bootstrap the frontend
                axum::response::Html(entry.into_value()).into_response()
            }
            Err(err) => {
                tracing::error!("Error rendering HTML: {:?}", err);
                StatusCode::INTERNAL_SERVER_ERROR.into_response()
            }
        }
    } else {
        // Return a 404 to crawlers that don't send `Accept: text/hml`.
        // This is to preserve legacy behavior and will likely change.
        // Most of these crawlers probably won't execute our frontend JS anyway, but
        // it would be nice to bootstrap the app for crawlers that do execute JS.
        StatusCode::NOT_FOUND.into_response()
    }
}

/// Extract the crate name from the path, by stripping [`PATH_PREFIX_CRATES`]
/// prefix, and returning the firsts path segment from the result.
/// Returns `None` if the path was not prefixed with [`PATH_PREFIX_CRATES`].
fn extract_crate_name(path: &str) -> Option<&str> {
    let suffix = path.strip_prefix(PATH_PREFIX_CRATES)?;
    let len = suffix.find('/').unwrap_or(suffix.len());
    let krate = &suffix[..len];
    krate.is_empty().not().then_some(krate)
}

/// Come up with an Open Graph image URL. In case a crate page is requested,
/// we use the crate's name as extracted from the request path and the OG image
/// base URL from config to generate one, otherwise we use the fallback image.
fn generate_og_image_url(path: &str, og_image_base_url: Option<&Url>) -> Option<Url> {
    let og_image_base_url = og_image_base_url?;

    let krate = extract_crate_name(path)?;

    let filename = format!("{krate}.png");
    og_image_base_url.join(&filename).ok()
}

#[cfg(test)]
mod tests {
    use googletest::{assert_that, prelude::eq};
    use url::Url;

    use crate::middleware::ember_html::{extract_crate_name, generate_og_image_url};

    #[test]
    fn test_extract_crate_name() {
        const PATHS: &[(&str, Option<&str>)] = &[
            ("/crates/tokio", Some("tokio")),
            ("/crates/tokio/versions", Some("tokio")),
            ("/crates/tokio/", Some("tokio")),
            ("/", None),
            ("/crates", None),
            ("/crates/", None),
            ("/dashboard/", None),
            ("/settings/profile", None),
        ];

        for (path, expected) in PATHS.iter().copied() {
            assert_that!(extract_crate_name(path), eq(expected));
        }
    }

    #[test]
    fn test_generate_og_image_url() {
        const PATHS: &[(&str, Option<&str>)] = &[
            ("/crates/tokio", Some("http://localhost:3000/og/tokio.png")),
            (
                "/crates/tokio/versions",
                Some("http://localhost:3000/og/tokio.png"),
            ),
            ("/crates/tokio/", Some("http://localhost:3000/og/tokio.png")),
            ("/", None),
            ("/crates", None),
            ("/crates/", None),
            ("/dashboard/", None),
            ("/settings/profile", None),
        ];

        let og_image_base_url: Url = "http://localhost:3000/og/".parse().unwrap();
        let og_image_base_url = Some(&og_image_base_url);

        for (path, expected) in PATHS.iter() {
            assert_eq!(
                generate_og_image_url(path, og_image_base_url),
                expected.map(|url| Url::parse(url).unwrap())
            );
        }
    }
}
