//! Render README files to HTML.

use crate::models::Version;
use crate::tasks::spawn_blocking;
use crate::worker::Environment;
use crates_io_markdown::text_to_html;
use crates_io_worker::BackgroundJob;
use diesel::result::DatabaseErrorKind;
use diesel::result::Error::DatabaseError;
use diesel_async::AsyncConnection;
use diesel_async::scoped_futures::ScopedFutureExt;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

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
        use diesel_async::RunQueryDsl;

        info!(version_id = ?self.version_id, "Rendering README");

        let job = self.clone();
        let rendered = spawn_blocking(move || {
            text_to_html(
                &job.text,
                &job.readme_path,
                job.base_url.as_deref(),
                job.pkg_path_in_vcs.as_ref(),
            )
        })
        .await?;

        if rendered.is_empty() {
            return Ok(());
        }

        let mut conn = env.deadpool.get().await?;
        conn.transaction(|conn| {
            async move {
                match Version::record_readme_rendering(job.version_id, conn).await {
                    Ok(_) => {}
                    Err(DatabaseError(DatabaseErrorKind::ForeignKeyViolation, ..)) => {
                        warn!(
                            "Skipping README rendering recording for version {}: version not found",
                            job.version_id
                        );
                        return Ok(());
                    }
                    Err(err) => {
                        warn!(
                            "Failed to record README rendering for version {}: {err}",
                            job.version_id,
                        );
                        return Err(err.into());
                    }
                }

                let result = versions::table
                    .find(job.version_id)
                    .inner_join(crates::table)
                    .select((crates::name, versions::num))
                    .first::<(String, String)>(conn)
                    .await
                    .optional()?;

                let Some((crate_name, vers)) = result else {
                    warn!(
                        "Skipping README rendering for version {}: version not found",
                        job.version_id
                    );
                    return Ok(());
                };

                tracing::Span::current().record("krate.name", tracing::field::display(&crate_name));

                let bytes = rendered.into();
                env.storage.upload_readme(&crate_name, &vers, bytes).await?;

                Ok(())
            }
            .scope_boxed()
        })
        .await
    }
}
