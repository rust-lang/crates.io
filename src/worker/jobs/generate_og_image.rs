use crate::models::OwnerKind;
use crate::schema::*;
use crate::worker::Environment;
use anyhow::Context;
use crates_io_og_image::{OgImageAuthorData, OgImageData};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::fs;
use tracing::{error, info, instrument, warn};

#[derive(Serialize, Deserialize)]
pub struct GenerateOgImage {
    crate_name: String,
    invalidate_cdns: bool,
}

impl GenerateOgImage {
    pub fn new(crate_name: String) -> Self {
        Self {
            crate_name,
            invalidate_cdns: true,
        }
    }

    pub fn without_cdn_invalidation(crate_name: String) -> Self {
        Self {
            crate_name,
            invalidate_cdns: false,
        }
    }
}

impl BackgroundJob for GenerateOgImage {
    const JOB_NAME: &'static str = "generate_og_image";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(krate.name = %self.crate_name))]
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let crate_name = &self.crate_name;

        let Some(option) = &ctx.og_image_generator else {
            warn!("OG image generator is not configured, skipping job for crate {crate_name}");
            return Ok(());
        };

        info!("Generating OG image for crate {crate_name}");

        let mut conn = ctx.deadpool.get().await?;

        // Fetch crate data
        let row = fetch_crate_data(crate_name, &mut conn).await;
        let row = row.context("Failed to fetch crate data")?;
        let Some(row) = row else {
            error!("Crate '{crate_name}' not found or has no default version");
            return Ok(());
        };

        let keywords: Vec<&str> = row.keywords.iter().flatten().map(|k| k.as_str()).collect();

        // Fetch user owners
        let owners = fetch_user_owners(row._crate_id, &mut conn).await;
        let owners = owners.context("Failed to fetch crate owners")?;
        let authors: Vec<OgImageAuthorData<'_>> = owners
            .iter()
            .map(|(login, avatar)| OgImageAuthorData::new(login, avatar.as_ref().map(Into::into)))
            .collect();

        // Build the OG image data
        let og_data = OgImageData {
            name: &row.crate_name,
            version: &row.version_num,
            description: row.description.as_deref(),
            license: row.license.as_deref(),
            tags: &keywords,
            authors: &authors,
            lines_of_code: None, // We don't track this yet
            crate_size: row.crate_size as u32,
            releases: row.num_versions as u32,
        };

        // Generate the OG image
        let temp_file = option.generate(og_data).await?;

        // Read the generated image
        let image_bytes = fs::read(temp_file.path()).await?;

        // Upload to storage
        ctx.storage
            .upload_og_image(crate_name, image_bytes.into())
            .await?;

        info!("Successfully generated and uploaded OG image for crate {crate_name}");

        if !self.invalidate_cdns {
            info!("Skipping CDN invalidation for crate {crate_name}");
            return Ok(());
        }

        // Invalidate CDN cache for the OG image
        let og_image_path = format!("og-images/{crate_name}.png");

        // Invalidate CloudFront CDN
        if let Some(cloudfront) = ctx.cloudfront() {
            if let Err(error) = cloudfront.invalidate(&og_image_path).await {
                warn!("Failed to invalidate CloudFront CDN for {crate_name}: {error}");
            }
        }

        // Invalidate Fastly CDN
        if let Some(fastly) = ctx.fastly() {
            if let Err(error) = fastly.invalidate(&og_image_path).await {
                warn!("Failed to invalidate Fastly CDN for {crate_name}: {error}");
            }
        }

        info!("CDN invalidation completed for crate {crate_name}");

        Ok(())
    }
}

#[derive(Queryable, Selectable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct QueryRow {
    #[diesel(select_expression = crates::id)]
    _crate_id: i32,
    #[diesel(select_expression = crates::name)]
    crate_name: String,
    #[diesel(select_expression = versions::num)]
    version_num: String,
    #[diesel(select_expression = versions::description)]
    description: Option<String>,
    #[diesel(select_expression = versions::license)]
    license: Option<String>,
    #[diesel(select_expression = versions::crate_size)]
    crate_size: i32,
    #[diesel(select_expression = versions::keywords)]
    keywords: Vec<Option<String>>,
    #[diesel(select_expression = default_versions::num_versions.assume_not_null())]
    num_versions: i32,
}

/// Fetches crate data and default version information by crate name
async fn fetch_crate_data(
    crate_name: &str,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Option<QueryRow>> {
    crates::table
        .inner_join(default_versions::table)
        .inner_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .filter(crates::name.eq(crate_name))
        .select(QueryRow::as_select())
        .first(conn)
        .await
        .optional()
}

/// Fetches user owners and their avatars for a crate by crate ID
async fn fetch_user_owners(
    crate_id: i32,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Vec<(String, Option<String>)>> {
    crate_owners::table
        .inner_join(users::table.on(crate_owners::owner_id.eq(users::id)))
        .filter(crate_owners::crate_id.eq(crate_id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .filter(crate_owners::deleted.eq(false))
        .select((users::gh_login, users::gh_avatar))
        .load(conn)
        .await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::builders::CrateBuilder;
    use crate::tests::util::TestApp;
    use claims::{assert_err, assert_ok};
    use crates_io_env_vars::var;
    use crates_io_worker::BackgroundJob;
    use insta::assert_binary_snapshot;
    use std::process::Command;

    fn is_ci() -> bool {
        var("CI").unwrap().is_some()
    }

    fn typst_available() -> bool {
        Command::new("typst").arg("--version").spawn().is_ok()
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_generate_og_image_job() {
        let (app, _, user) = TestApp::full().with_og_image_generator().with_user().await;

        if !is_ci() && !typst_available() {
            warn!("Skipping OG image generation test because 'typst' is not available");
            return;
        }

        let mut conn = app.db_conn().await;

        // Create a test crate with keywords using CrateBuilder
        CrateBuilder::new("test-crate", user.as_model().id)
            .description("A test crate for OG image generation")
            .keyword("testing")
            .keyword("rust")
            .expect_build(&mut conn)
            .await;

        // Create and enqueue the job
        let job = GenerateOgImage::new("test-crate".to_string());
        job.enqueue(&mut conn).await.unwrap();

        // Run the background job
        app.run_pending_background_jobs().await;

        // Verify the OG image was uploaded to storage
        let storage = app.as_inner().storage.as_inner();
        let og_image_path = "og-images/test-crate.png";

        // Try to download the image to verify it exists
        let download_result = storage.get(&og_image_path.into()).await;
        let result = assert_ok!(
            download_result,
            "OG image should be uploaded to storage at: {og_image_path}"
        );

        // Verify it's a non-empty file
        let image_bytes = result.bytes().await.unwrap().to_vec();
        assert!(!image_bytes.is_empty(), "OG image should not be empty");

        // Verify it starts with PNG magic bytes
        assert_eq!(
            &image_bytes[0..8],
            &[0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A],
            "Uploaded file should be a valid PNG"
        );

        assert_binary_snapshot!("og-image.png", image_bytes);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_generate_og_image_job_nonexistent_crate() {
        let (app, _, _) = TestApp::full().with_user().await;
        let mut conn = app.db_conn().await;

        // Create and enqueue the job for a non-existent crate
        let job = GenerateOgImage::new("nonexistent-crate".to_string());
        job.enqueue(&mut conn).await.unwrap();

        // Run the background job - should complete without error
        app.run_pending_background_jobs().await;

        // Verify no OG image was uploaded
        let storage = app.as_inner().storage.as_inner();
        let og_image_path = "og-images/nonexistent-crate.png";
        let download_result = storage.get(&og_image_path.into()).await;
        assert_err!(
            download_result,
            "No OG image should be uploaded for nonexistent crate"
        );
    }
}
