//! Render README files to HTML.

use crate::swirl::PerformError;
use anyhow::Context;
use crates_io_markdown::text_to_html;

use crate::background_jobs::{BackgroundJob, Environment, PerformState};
use crate::models::Version;

#[derive(Serialize, Deserialize)]
pub struct RenderAndUploadReadmeJob {
    pub(crate) version_id: i32,
    pub(crate) text: String,
    pub(crate) readme_path: String,
    pub(crate) base_url: Option<String>,
    pub(crate) pkg_path_in_vcs: Option<String>,
}

impl BackgroundJob for RenderAndUploadReadmeJob {
    const JOB_NAME: &'static str = "render_and_upload_readme";

    #[instrument(skip_all, fields(krate.name))]
    fn run(&self, state: PerformState<'_>, env: &Environment) -> Result<(), PerformError> {
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
