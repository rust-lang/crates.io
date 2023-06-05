//! Render README files to HTML.

use crate::swirl::PerformError;
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

    conn.transaction(|conn| {
        Version::record_readme_rendering(version_id, conn)?;
        let (crate_name, vers): (String, String) = versions::table
            .find(version_id)
            .inner_join(crates::table)
            .select((crates::name, versions::num))
            .first(conn)?;

        tracing::Span::current().record("krate.name", tracing::field::display(&crate_name));

        env.uploader
            .upload_readme(env.http_client(), &crate_name, &vers, rendered)?;
        Ok(())
    })
}
