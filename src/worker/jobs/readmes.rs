//! Render README files to HTML.

use crate::models::Version;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_markdown::text_to_html;
use crates_io_worker::BackgroundJob;
use std::sync::Arc;
use tokio::runtime::Handle;

#[derive(Clone, Serialize, Deserialize)]
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
    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        use crate::schema::*;
        use diesel::prelude::*;

        info!(version_id = ?self.version_id, "Rendering README");

        let job = self.clone();
        let rendered = spawn_blocking(move || {
            Ok::<_, anyhow::Error>(text_to_html(
                &job.text,
                &job.readme_path,
                job.base_url.as_deref(),
                job.pkg_path_in_vcs.as_ref(),
            ))
        })
        .await?;

        if rendered.is_empty() {
            return Ok(());
        }

        let conn = env.deadpool.get().await?;
        conn.interact(move |conn| {
            conn.transaction(|conn| {
                Version::record_readme_rendering(job.version_id, conn)?;
                let (crate_name, vers): (String, String) = versions::table
                    .find(job.version_id)
                    .inner_join(crates::table)
                    .select((crates::name, versions::num))
                    .first(conn)?;

                tracing::Span::current().record("krate.name", tracing::field::display(&crate_name));

                let bytes = rendered.into();
                let future = env.storage.upload_readme(&crate_name, &vers, bytes);
                Handle::current().block_on(future)?;

                Ok(())
            })
        })
        .await
        .map_err(|err| anyhow!(err.to_string()))?
    }
}
