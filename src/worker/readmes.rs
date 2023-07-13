//! Render README files to HTML.

use crate::swirl::PerformError;
use anyhow::Context;
use crates_io_markdown::text_to_html;
use diesel::PgConnection;

use crate::background_jobs::Environment;
use crate::models::Version;

#[instrument(skip_all, fields(krate.name))]
pub fn perform_render_and_upload_readme(
    conn: &mut PgConnection,
    env: &Environment,
    version_id: i32,
    text: &str,
    readme_path: &str,
    base_url: Option<&str>,
    pkg_path_in_vcs: Option<&str>,
) -> Result<(), PerformError> {
    use crate::schema::*;
    use diesel::prelude::*;

    info!(?version_id, "Rendering README");

    let rendered = text_to_html(text, readme_path, base_url, pkg_path_in_vcs);
    if rendered.is_empty() {
        return Ok(());
    }

    conn.transaction(|conn| {
        Version::record_readme_rendering(version_id, conn)?;
        let (crate_name, vers): (String, String) = versions::table
            .find(version_id)
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
