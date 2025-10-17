use crate::models::OwnerKind;
use crate::schema::*;
use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use bigdecimal::ToPrimitive;
use crates_io_database::models::CloudFrontInvalidationQueueItem;
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
            lines_of_code: row.total_code_lines(),
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

        // Queue CloudFront invalidation for batch processing
        if ctx.cloudfront().is_some() {
            let paths = std::slice::from_ref(&og_image_path);
            let result = CloudFrontInvalidationQueueItem::queue_paths(&mut conn, paths).await;
            if let Err(error) = result {
                warn!("Failed to queue CloudFront invalidation for {crate_name}: {error}");
            } else if let Err(error) = ProcessCloudfrontInvalidationQueue.enqueue(&mut conn).await {
                warn!(
                    "Failed to enqueue CloudFront invalidation processing job for {crate_name}: {error}"
                );
            }
        }

        // Invalidate Fastly CDN
        if let Some(fastly) = ctx.fastly()
            && let Err(error) = fastly.invalidate(&og_image_path).await
        {
            warn!("Failed to invalidate Fastly CDN for {crate_name}: {error}");
        }

        info!("CDN invalidation completed for crate {crate_name}");

        Ok(())
    }
}

#[derive(HasQuery)]
#[diesel(
    base_query = crates::table
        .inner_join(default_versions::table)
        .inner_join(versions::table.on(default_versions::version_id.eq(versions::id)))
)]
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
    #[diesel(select_expression = versions::linecounts.retrieve_as_object("total_code_lines"))]
    total_code_lines: Option<serde_json::Value>,
    #[diesel(select_expression = default_versions::num_versions.assume_not_null())]
    num_versions: i32,
}

impl QueryRow {
    fn total_code_lines(&self) -> Option<u32> {
        self.total_code_lines
            .as_ref()
            .and_then(serde_json::Value::as_u64)
            .as_ref()
            .and_then(ToPrimitive::to_u32)
    }
}

/// Fetches crate data and default version information by crate name
async fn fetch_crate_data(
    crate_name: &str,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Option<QueryRow>> {
    QueryRow::query()
        .filter(crates::name.eq(crate_name))
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
