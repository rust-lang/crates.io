use crate::schema::{cache_tags_backfills, crates, versions};
use crate::storage::StorageKey;
use crate::worker::Environment;
use anyhow::Context;
use aws_sdk_s3::config::retry::RetryConfig;
use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use aws_sdk_s3::error::{ProvideErrorMetadata, SdkError};
use aws_sdk_s3::operation::copy_object::CopyObjectError;
use aws_sdk_s3::types::MetadataDirective;
use aws_sdk_s3::{Client, Config};
use crates_io_database::models::NewCacheTagsBackfillRow;
use crates_io_env_vars::{required_var, var};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{info, instrument, warn};

/// Copies every S3 object for a single crate over itself to attach the
/// `cache-tags` metadata that new uploads already carry, then records the
/// completion in `cache_tags_backfills`.
#[derive(Serialize, Deserialize)]
pub struct BackfillCacheTags {
    name: String,
}

impl BackfillCacheTags {
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

impl BackgroundJob for BackfillCacheTags {
    const JOB_NAME: &'static str = "backfill_cache_tags";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    #[instrument(skip_all, fields(crate.name = %self.name))]
    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        if !ctx.config.features.cache_tags_enabled {
            warn!("`CACHE_TAGS_ENABLED` is not set, skipping backfill job");
            return Ok(());
        }

        let name = &self.name;
        let mut conn = ctx.deadpool.get().await?;

        let crate_id = crates::table
            .filter(crates::name.eq(name))
            .select(crates::id)
            .first::<i32>(&mut conn)
            .await
            .optional()
            .context("Failed to look up crate")?;

        let Some(crate_id) = crate_id else {
            warn!("Crate not found, skipping cache-tags backfill");
            return Ok(());
        };

        let nums = versions::table
            .filter(versions::crate_id.eq(crate_id))
            .select(versions::num)
            .load::<String>(&mut conn)
            .await
            .context("Failed to load crate versions")?;

        let keys = objects_to_backfill(name, &nums);
        let total = keys.len();
        info!("Backfilling cache-tags for {total} objects…");

        let (client, bucket) = s3_client()?;

        let mut copied = 0;
        for key in &keys {
            if copy_with_cache_tags(&client, &bucket, key).await? {
                copied += 1;
            }
        }
        info!("Backfilled cache-tags for {copied}/{total} objects");

        record_completion(&mut conn, crate_id, name).await?;

        Ok(())
    }
}

/// Returns every [`StorageKey`] that should carry cache-tags for the given
/// crate and versions.
fn objects_to_backfill<'a>(name: &'a str, versions: &'a [String]) -> Vec<StorageKey<'a>> {
    let mut keys = Vec::with_capacity(versions.len() * 4 + 2);
    for version in versions {
        keys.push(StorageKey::for_crate_file(name, version));
        keys.push(StorageKey::for_crate_zip(name, version));
        keys.push(StorageKey::for_crate_zip_manifest(name, version));
        keys.push(StorageKey::for_readme(name, version));
    }
    keys.push(StorageKey::for_og_image(name));
    keys.push(StorageKey::CrateFeed { name });
    keys
}

/// Copies a single object over itself with `MetadataDirective=Replace`, which
/// drops and re-supplies the full metadata set, so we re-send the content-type
/// and cache-control alongside the new `cache-tags`. Returns `Ok(false)` when
/// the source object does not exist.
async fn copy_with_cache_tags(
    client: &Client,
    bucket: &str,
    key: &StorageKey<'_>,
) -> anyhow::Result<bool> {
    let path = key.path();
    let path = path.as_ref();

    let mut request = client
        .copy_object()
        .bucket(bucket)
        .key(path)
        .copy_source(format!("{bucket}/{path}"))
        .metadata_directive(MetadataDirective::Replace);

    if let Some(content_type) = key.content_type() {
        request = request.content_type(content_type);
    }
    if let Some(cache_control) = key.cache_control() {
        request = request.cache_control(cache_control);
    }
    if let Some(cache_tags) = key.cache_tags() {
        request = request.metadata("cache-tags", cache_tags);
    }

    match request.send().await {
        Ok(_) => Ok(true),
        Err(err) if is_not_found(&err) => Ok(false),
        Err(err) => Err(err).with_context(|| format!("Failed to copy {path}")),
    }
}

/// Whether a `CopyObject` failure is a missing source object.
fn is_not_found(err: &SdkError<CopyObjectError>) -> bool {
    err.as_service_error().and_then(|err| err.code()) == Some("NoSuchKey")
}

/// Records that the crate's backfill completed, refreshing the timestamp if a
/// record already exists.
async fn record_completion(
    conn: &mut AsyncPgConnection,
    crate_id: i32,
    name: &str,
) -> anyhow::Result<()> {
    let row = NewCacheTagsBackfillRow::builder()
        .crate_id(crate_id)
        .crate_name(name)
        .build();

    diesel::insert_into(cache_tags_backfills::table)
        .values(row)
        .on_conflict(cache_tags_backfills::crate_id)
        .do_update()
        .set(cache_tags_backfills::completed_at.eq(diesel::dsl::now))
        .execute(conn)
        .await
        .context("Failed to record cache-tags backfill completion")?;

    Ok(())
}

/// Builds an S3 client and returns it with the target bucket name, read from the
/// same environment variables the storage layer uses.
fn s3_client() -> anyhow::Result<(Client, String)> {
    let bucket = required_var("S3_BUCKET")?;
    let region = var("S3_REGION")?.unwrap_or_else(|| "us-west-1".to_string());
    let access_key = required_var("AWS_ACCESS_KEY")?;
    let secret_key = required_var("AWS_SECRET_KEY")?;

    let credentials = Credentials::from_keys(access_key, secret_key, None);

    let config = Config::builder()
        .behavior_version(BehaviorVersion::v2026_01_12())
        .region(Region::new(region))
        .credentials_provider(credentials)
        .retry_config(RetryConfig::standard().with_max_attempts(10))
        .build();

    Ok((Client::from_conf(config), bucket))
}

#[cfg(test)]
mod tests {
    use super::*;
    use claims::assert_some;
    use insta::assert_snapshot;

    #[test]
    fn objects_to_backfill_lists_every_versioned_and_crate_level_object() {
        let versions = vec!["1.0.0".to_string(), "2.0.0+build.1".to_string()];
        let keys = objects_to_backfill("foo", &versions);

        let paths = keys
            .iter()
            .map(|key| key.path().as_ref().to_string())
            .collect::<Vec<_>>();

        assert_snapshot!(paths.join("\n"), @"
        crates/foo/foo-1.0.0.crate
        crates/foo/foo-1.0.0.zip
        crates/foo/foo-1.0.0.zip.json
        readmes/foo/foo-1.0.0.html
        crates/foo/foo-2.0.0+build.1.crate
        crates/foo/foo-2.0.0+build.1.zip
        crates/foo/foo-2.0.0+build.1.zip.json
        readmes/foo/foo-2.0.0+build.1.html
        og-images/foo.png
        rss/crates/foo.xml
        ");

        // Every object should carry cache-tags.
        for key in &keys {
            assert_some!(key.cache_tags());
        }
    }
}
