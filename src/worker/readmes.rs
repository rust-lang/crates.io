//! Render README files to HTML.

use crate::swirl::PerformError;
use anyhow::Context;
use crates_io_markdown::text_to_html;
use diesel::PgConnection;

use crate::background_jobs::Environment;
use crate::models::Version;

#[derive(Serialize, Deserialize)]
pub struct RenderAndUploadReadmeJob {
    pub(crate) version_id: i32,
    pub(crate) text: String,
    pub(crate) readme_path: String,
    pub(crate) base_url: Option<String>,
    pub(crate) pkg_path_in_vcs: Option<String>,
}

#[instrument(skip_all, fields(krate.name))]
pub fn perform_render_and_upload_readme(
    job: &RenderAndUploadReadmeJob,
    conn: &mut PgConnection,
    env: &Environment,
) -> Result<(), PerformError> {
    use crate::schema::*;
    use diesel::prelude::*;

    info!(version_id = ?job.version_id, "Rendering README");

    let rendered = text_to_html(
        &job.text,
        &job.readme_path,
        job.base_url.as_deref(),
        job.pkg_path_in_vcs.as_ref(),
    );
    if rendered.is_empty() {
        return Ok(());
    }

    conn.transaction(|conn| {
        Version::record_readme_rendering(job.version_id, conn)?;
        let (crate_name, vers): (String, String) = versions::table
            .find(job.version_id)
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
