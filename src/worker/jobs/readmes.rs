//! Render README files to HTML.

use crate::models::Version;
use crate::worker::swirl::{BackgroundJob, PerformState};
use crate::worker::Environment;
use anyhow::Context;
use crates_io_markdown::text_to_html;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct RenderAndUploadReadme {
    version_id: i32,
    text: String,
    readme_path: String,
    base_url: Option<String>,
    pkg_path_in_vcs: Option<String>,
}

impl RenderAndUploadReadme {
    pub fn new(
        version_id: i32,
        text: String,
        readme_path: String,
        base_url: Option<String>,
        pkg_path_in_vcs: Option<String>,
    ) -> Self {
        Self {
            version_id,
            text,
            readme_path,
            base_url,
            pkg_path_in_vcs,
        }
    }
}

impl BackgroundJob for RenderAndUploadReadme {
    const JOB_NAME: &'static str = "render_and_upload_readme";
    const PRIORITY: i16 = 50;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(krate.name))]
    fn run(&self, state: PerformState<'_>, env: &Self::Context) -> anyhow::Result<()> {
        use crate::schema::*;
        use diesel::prelude::*;

        info!(version_id = ?self.version_id, "Rendering README");

        let rendered = text_to_html(
            &self.text,
            &self.readme_path,
            self.base_url.as_deref(),
            self.pkg_path_in_vcs.as_ref(),
        );
        if rendered.is_empty() {
            return Ok(());
        }

        state.conn.transaction(|conn| {
            Version::record_readme_rendering(self.version_id, conn)?;
            let (crate_name, vers): (String, String) = versions::table
                .find(self.version_id)
                .inner_join(crates::table)
                .select((crates::name, versions::num))
                .first(conn)?;

            tracing::Span::current().record("krate.name", tracing::field::display(&crate_name));

            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .context("Failed to initialize tokio runtime")
                .unwrap();

            let bytes = rendered.into();
            let future = env.storage.upload_readme(&crate_name, &vers, bytes);
            rt.block_on(future)?;

            Ok(())
        })
    }
}
