use crates_io_env_vars::var_parsed;
use url::Url;

#[derive(Debug)]
pub struct FrontendConfig {
    /// Should the server serve the frontend assets in the `dist` directory?
    pub serve_dist: bool,

    /// Should the server serve the frontend `index.html` for all
    /// non-API requests?
    pub serve_html: bool,

    /// Base URL for the service from which the OpenGraph images
    /// for crates are loaded. Required if
    /// [`Self::serve_html`] is set.
    ///
    /// Read from the `OG_IMAGE_BASE_URL` environment variable.
    pub og_image_base_url: Option<Url>,

    /// Maximum number of items that the HTML render
    /// cache in `crate::middleware::frontend_html::serve`
    /// can hold. Defaults to 1024.
    ///
    /// Read from the `HTML_RENDER_CACHE_CAP` environment variable.
    pub html_render_cache_max_capacity: u64,
}

impl FrontendConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        Ok(Self {
            serve_dist: true,
            serve_html: true,
            og_image_base_url: var_parsed("OG_IMAGE_BASE_URL")?,
            html_render_cache_max_capacity: var_parsed("HTML_RENDER_CACHE_CAP")?.unwrap_or(1024),
        })
    }
}
