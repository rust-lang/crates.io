use crate::models::OwnerKind;
use crate::schema::*;
use crate::storage::StorageKey;
use crate::worker::Environment;
use crate::worker::jobs::ProcessCloudfrontInvalidationQueue;
use anyhow::Context;
use bigdecimal::ToPrimitive;
use crates_io_database::models::{CloudFrontDistribution, CloudFrontInvalidationQueueItem};
use crates_io_og_image::{OgImageAuthorData, OgImageData};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
    async fn run(self, ctx: Self::Context) -> anyhow::Result<()> {
        let crate_name = &self.crate_name;

        let Some(option) = &ctx.og_image_generator else {
            warn!("OG image generator is not configured, skipping job for crate {crate_name}");
            return Ok(());
        };

        info!("Generating OG image for crate {crate_name}");

        let conn = ctx.deadpool.get().await?;

        // Fetch crate data
        let row = fetch_crate_data(crate_name, &conn).await;
        let row = row.context("Failed to fetch crate data")?;
        let Some(row) = row else {
            error!("Crate '{crate_name}' not found or has no default version");
            return Ok(());
        };

        let keywords: Vec<&str> = row.keywords.iter().flatten().map(|k| k.as_str()).collect();

        // Fetch user owners
        let owners = fetch_user_owners(row._crate_id, &conn).await;
        let owners = owners.context("Failed to fetch crate owners")?;

        let authors = build_og_author_data(&owners);

        let og_data = build_og_image_data(&row, &keywords, &authors);

        // Generate the OG image
        let image_bytes = option.generate(og_data).await?;

        // Upload to storage
        let key = StorageKey::for_og_image(crate_name);
        ctx.storage.upload(&key, image_bytes.into()).await?;

        info!("Successfully generated and uploaded OG image for crate {crate_name}");

        if !self.invalidate_cdns {
            info!("Skipping CDN invalidation for crate {crate_name}");
            return Ok(());
        }

        // Invalidate CDN cache for the OG image
        let og_image_path = key.cdn_path();

        // Queue CloudFront invalidation for batch processing
        if ctx.cloudfront().is_some() {
            let distribution = CloudFrontDistribution::Static;
            let paths = std::slice::from_ref(&og_image_path);
            let result =
                CloudFrontInvalidationQueueItem::queue_paths(&conn, distribution, paths).await;
            if let Err(error) = result {
                warn!("Failed to queue CloudFront invalidation for {crate_name}: {error}");
            } else if let Err(error) = ProcessCloudfrontInvalidationQueue.enqueue(&conn).await {
                warn!(
                    "Failed to enqueue CloudFront invalidation processing job for {crate_name}: {error}"
                );
            }
        }

        // Invalidate Fastly CDN
        if let Some(fastly) = ctx.fastly()
            && let Some(cdn_domain) = &ctx.config.storage.cdn_prefix
            && let Err(error) = fastly.purge_both_domains(cdn_domain, &og_image_path).await
        {
            warn!("Failed to invalidate Fastly CDN for {crate_name}: {error}");
        }

        info!("CDN invalidation completed for crate {crate_name}");

        Ok(())
    }
}

#[derive(HasQuery, PartialEq, Debug)]
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
    mut conn: &AsyncPgConnection,
) -> QueryResult<Option<QueryRow>> {
    QueryRow::query()
        .filter(crates::name.eq(crate_name))
        .first(&mut conn)
        .await
        .optional()
}

/// Fetches user owners and their avatars for a crate by crate ID
async fn fetch_user_owners(
    crate_id: i32,
    mut conn: &AsyncPgConnection,
) -> QueryResult<Vec<(String, Option<String>)>> {
    crate_owners::table
        .inner_join(users::table.on(crate_owners::owner_id.eq(users::id)))
        .left_join(oauth_github::table.on(users::id.eq(oauth_github::user_id)))
        .filter(crate_owners::crate_id.eq(crate_id))
        .filter(crate_owners::owner_kind.eq(OwnerKind::User))
        .filter(crate_owners::deleted.eq(false))
        .select((users::gh_login, oauth_github::avatar.nullable()))
        .load(&mut conn)
        .await
}

fn build_og_author_data<'a>(owners: &'a [(String, Option<String>)]) -> Vec<OgImageAuthorData<'a>> {
    owners
        .iter()
        .map(|(login, avatar)| OgImageAuthorData::new(login, avatar.as_ref().map(Into::into)))
        .collect()
}

fn build_og_image_data<'a>(
    row: &'a QueryRow,
    keywords: &'a [&'a str],
    authors: &'a [OgImageAuthorData<'a>],
) -> OgImageData<'a> {
    OgImageData {
        name: &row.crate_name,
        version: &row.version_num,
        description: row.description.as_deref(),
        license: row.license.as_deref(),
        tags: keywords,
        authors,
        lines_of_code: row.total_code_lines(),
        crate_size: row.crate_size as u32,
        releases: row.num_versions as u32,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_database::models::{OauthGithub, User};
    use crates_io_test_db::TestDatabase;
    use crates_io_test_utils::builders::{CrateBuilder, OauthGithubBuilder, UserBuilder};

    #[tokio::test]
    async fn fetch_crate_info() {
        let db = TestDatabase::new();
        let mut conn = db.async_connect().await;

        let new_user = UserBuilder::new().with_username("foo").new_user();
        let user_id = new_user.insert(&conn).await.unwrap();
        let user = User::find(&conn, user_id).await.unwrap();
        OauthGithubBuilder::for_user(&user)
            .with_avatar("http://example.com/icon-the-first.png")
            .insert(&conn)
            .await;
        let oauth_github = OauthGithub::belonging_to(&user)
            .select(OauthGithub::as_select())
            .first(&mut conn)
            .await
            .unwrap();

        let crate_name = "test-crate";
        let crate_description = "A test crate for OG image generation";
        let test_crate = CrateBuilder::new(crate_name, user_id)
            .description(crate_description)
            .keyword("testing")
            .keyword("rust")
            .expect_build(&mut conn)
            .await;

        let row = fetch_crate_data(crate_name, &conn).await.unwrap().unwrap();

        assert_eq!(
            row,
            QueryRow {
                _crate_id: test_crate.id,
                crate_name: crate_name.to_string(),
                version_num: "0.99.0".to_string(),
                description: Some(crate_description.to_string()),
                license: Some("MIT".to_string()),
                crate_size: 4242,
                keywords: vec![Some("testing".to_string()), Some("rust".to_string())],
                total_code_lines: Some(serde_json::Value::Number(serde_json::Number::from(9000))),
                num_versions: 1,
            }
        );

        let user_owners = fetch_user_owners(test_crate.id, &conn).await.unwrap();
        assert_eq!(
            user_owners,
            vec![(
                "foo".to_string(),
                Some("http://example.com/icon-the-first.png".to_string())
            )],
        );

        let authors = build_og_author_data(&user_owners);
        assert_eq!(authors.len(), 1);
        assert_eq!(authors[0].name, "foo");
        assert_eq!(
            authors[0].avatar.as_ref().unwrap(),
            &oauth_github.avatar.unwrap()
        );

        let keywords = &["testing", "rust"];
        let og_image_data = build_og_image_data(&row, keywords, &authors);

        assert_eq!(og_image_data.name, crate_name);
        assert_eq!(og_image_data.version, "0.99.0");
        assert_eq!(og_image_data.description, Some(crate_description));
        assert_eq!(og_image_data.license, Some("MIT"));
        assert_eq!(og_image_data.tags, keywords);
        assert_eq!(og_image_data.crate_size, 4242);
        assert_eq!(og_image_data.releases, 1);
    }
}
