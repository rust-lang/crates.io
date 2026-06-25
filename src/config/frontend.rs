use crates_io_env_vars::var_parsed;

#[derive(Debug)]
pub struct FrontendConfig {
    /// Should the server serve the frontend assets in the `dist` directory?
    pub serve_dist: bool,

    /// Should the server serve the frontend `index.html` for all
    /// non-API requests?
    pub serve_html: bool,

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
            html_render_cache_max_capacity: var_parsed("HTML_RENDER_CACHE_CAP")?.unwrap_or(1024),
        })
    }
}
