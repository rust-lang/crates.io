//! Render README files to HTML.

use cargo_registry_markdown::text_to_html;
use swirl::PerformError;

use crate::background_jobs::Environment;
use crate::models::Version;

#[swirl::background_job]
pub fn render_and_upload_readme(
    conn: &PgConnection,
    env: &Environment,
    version_id: i32,
    text: String,
    readme_path: String,
    base_url: Option<String>,
    pkg_path_in_vcs: Option<String>,
) -> Result<(), PerformError> {
    use crate::schema::*;
    use diesel::prelude::*;

    let rendered = text_to_html(
        &text,
        &readme_path,
        base_url.as_deref(),
        pkg_path_in_vcs.as_deref(),
    );

    conn.transaction(|| {
        Version::record_readme_rendering(version_id, conn)?;
        let (crate_name, vers): (String, String) = versions::table
            .find(version_id)
            .inner_join(crates::table)
            .select((crates::name, versions::num))
            .first(&*conn)?;
        env.uploader
            .upload_readme(env.http_client(), &crate_name, &vers, rendered)?;
        Ok(())
    })
}
