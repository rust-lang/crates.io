use crate::config::CdnLogStorageConfig;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::worker::Environment;
use anyhow::Context;
use chrono::NaiveDate;
use crates_io_cdn_logs::{count_downloads, Decompressor, DownloadsMap};
use crates_io_worker::BackgroundJob;
use diesel::dsl::exists;
use diesel::prelude::*;
use diesel::{select, QueryResult};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use object_store::aws::AmazonS3Builder;
use object_store::local::LocalFileSystem;
use object_store::memory::InMemory;
use object_store::path::Path;
use object_store::ObjectStore;
use semver::Version;
use std::fmt::Debug;
use std::sync::Arc;
use tokio::io::BufReader;

/// A background job that loads a CDN log file from an object store (aka. S3),
/// counts the number of downloads for each crate and version, and then inserts
/// the results into the database.
#[derive(Debug, Serialize, Deserialize)]
pub struct ProcessCdnLog {
    pub region: String,
    pub bucket: String,
    pub path: String,
}

impl ProcessCdnLog {
    pub fn new(region: String, bucket: String, path: String) -> Self {
        Self {
            region,
            bucket,
            path,
        }
    }
}

impl BackgroundJob for ProcessCdnLog {
    const JOB_NAME: &'static str = "process_cdn_log";
    const DEDUPLICATED: bool = true;
    const QUEUE: &'static str = "downloads";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        // The store is rebuilt for each run because we don't want to assume
        // that all log files live in the same AWS region or bucket, and those
        // two pieces are necessary for the store construction.
        let store = build_store(&ctx.config.cdn_log_storage, &self.region, &self.bucket)
            .context("Failed to build object store")?;

        let db_pool = ctx.deadpool.clone();
        run(store, &self.path, db_pool).await
    }
}

/// Builds an object store based on the [CdnLogStorageConfig] and the
/// `region` and `bucket` arguments.
///
/// If the passed in [CdnLogStorageConfig] is using local file or in-memory
/// storage the `region` and `bucket` arguments are ignored.
fn build_store(
    config: &CdnLogStorageConfig,
    region: impl Into<String>,
    bucket: impl Into<String>,
) -> anyhow::Result<Arc<dyn ObjectStore>> {
    match config {
        CdnLogStorageConfig::S3 {
            access_key,
            secret_key,
        } => {
            use secrecy::ExposeSecret;

            let store = AmazonS3Builder::new()
                .with_region(region.into())
                .with_bucket_name(bucket.into())
                .with_access_key_id(access_key)
                .with_secret_access_key(secret_key.expose_secret())
                .build()?;

            Ok(Arc::new(store))
        }
        CdnLogStorageConfig::Local { path } => {
            Ok(Arc::new(LocalFileSystem::new_with_prefix(path)?))
        }
        CdnLogStorageConfig::Memory => Ok(Arc::new(InMemory::new())),
    }
}

/// Loads the given log file from the object store and counts the number of
/// downloads for each crate and version. The results are printed to the log.
///
/// This function is separate from the [`BackgroundJob`] trait method so that
/// it can be tested without having to construct a full [`Environment`]
/// struct.
#[instrument(skip_all, fields(cdn_log_store.path = %path))]
async fn run(
    store: Arc<dyn ObjectStore>,
    path: &str,
    db_pool: Pool<AsyncPgConnection>,
) -> anyhow::Result<()> {
    if already_processed(path, db_pool.clone()).await? {
        warn!("Skipping already processed log file");
        return Ok(());
    }

    let parsed_path =
        Path::parse(path).with_context(|| format!("Failed to parse path: {path:?}"))?;

    let downloads = load_and_count(&parsed_path, store).await?;
    if downloads.is_empty() {
        info!("No downloads found in log file");
        return Ok(());
    }

    log_stats(&downloads);

    let path = path.to_string();
    let conn = db_pool.get().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        conn.transaction(|conn| {
            // Mark the log file as processed before saving the downloads to
            // the database.
            //
            // If a second job is already processing the same log file, this
            // call will block until the second job has finished its
            // transaction and marked the log file as processed. Afterward
            // this call will throw a uniqueness error and fail the job.
            // When the job is retried the `already_processed()` call above
            // will return `true` and the job will skip processing the log
            // file again.
            save_as_processed(path, conn)?;

            save_downloads(downloads, conn)
        })?;

        Ok::<_, anyhow::Error>(())
    })
    .await
}

/// Loads the given log file from the object store and counts the number of
/// downloads for each crate and version.
async fn load_and_count(path: &Path, store: Arc<dyn ObjectStore>) -> anyhow::Result<DownloadsMap> {
    let meta = store.head(path).await;
    let meta = meta.with_context(|| format!("Failed to request metadata for {path:?}"))?;

    let reader = object_store::buffered::BufReader::new(store, &meta);
    let decompressor = Decompressor::from_extension(reader, path.extension())?;
    let reader = BufReader::new(decompressor);

    count_downloads(reader).await
}

/// Prints the total number of downloads, the number of crates, and the number
/// of needed inserts to the log.
fn log_stats(downloads: &DownloadsMap) {
    let total_downloads = downloads.sum_downloads();
    info!("Total number of downloads: {total_downloads}");

    let num_crates = downloads.unique_crates().len();
    info!("Number of crates: {num_crates}");

    let total_inserts = downloads.len();
    info!("Number of needed inserts: {total_inserts}");
}

table! {
    /// Diesel table definition for the temporary `temp_downloads` table that is
    /// created by the [`create_temp_downloads_table`] function.
    ///
    /// The primary key does not actually exist, but specifying one is
    /// required by Diesel.
    temp_downloads (name, version, date) {
        name -> Text,
        version -> Text,
        date -> Date,
        downloads -> BigInt,
    }
}

/// Helper struct for inserting downloads into the `temp_downloads` table.
#[derive(Insertable)]
#[diesel(table_name = temp_downloads)]
struct NewDownload {
    name: String,
    version: String,
    date: NaiveDate,
    downloads: i64,
}

impl From<(String, Version, NaiveDate, u64)> for NewDownload {
    fn from((name, version, date, downloads): (String, Version, NaiveDate, u64)) -> Self {
        Self {
            name,
            version: version.to_string(),
            date,
            downloads: downloads as i64,
        }
    }
}

/// Saves the downloads from the given [`DownloadsMap`] to the database into
/// the `version_downloads` table.
///
/// This function **should be run inside a transaction** to ensure that the
/// temporary `temp_downloads` table is dropped after the inserts are
/// completed!
///
/// The temporary table only exists on the current connection, but if a
/// connection pool is used, the temporary table will not be dropped when
/// the connection is returned to the pool.
pub fn save_downloads(downloads: DownloadsMap, conn: &mut impl Conn) -> anyhow::Result<()> {
    debug!("Creating temp_downloads table");
    create_temp_downloads_table(conn).context("Failed to create temp_downloads table")?;

    debug!("Saving counted downloads to temp_downloads table");
    fill_temp_downloads_table(downloads, conn).context("Failed to fill temp_downloads table")?;

    debug!("Saving temp_downloads to version_downloads table");
    let failed_inserts = save_to_version_downloads(conn)
        .context("Failed to save temp_downloads to version_downloads table")?;

    if !failed_inserts.is_empty() {
        warn!(
            "Failed to insert downloads for the following crates and versions: {failed_inserts:?}"
        );
    }

    Ok(())
}

/// Creates the temporary `temp_downloads` table that is used to store the
/// counted downloads before they are inserted into the `version_downloads`
/// table.
///
/// We can't insert directly into `version_downloads` table because we need to
/// look up the `version_id` for each crate and version combination, and that
/// requires a join with the `crates` and `versions` tables.
#[instrument("db.query", skip_all, fields(message = "CREATE TEMPORARY TABLE ..."))]
fn create_temp_downloads_table(conn: &mut impl Conn) -> QueryResult<usize> {
    diesel::sql_query(
        r#"
            CREATE TEMPORARY TABLE temp_downloads (
                name VARCHAR NOT NULL,
                version VARCHAR NOT NULL,
                date DATE NOT NULL,
                downloads INTEGER NOT NULL
            ) ON COMMIT DROP;
        "#,
    )
    .execute(conn)
}

/// Fills the temporary `temp_downloads` table with the downloads from the
/// given [`DownloadsMap`].
#[instrument(
    "db.query",
    skip_all,
    fields(message = "INSERT INTO temp_downloads ...")
)]
fn fill_temp_downloads_table(downloads: DownloadsMap, conn: &mut impl Conn) -> QueryResult<()> {
    // `tokio-postgres` has a limit on the size of values it can send to the
    // database. To avoid hitting this limit, we insert the downloads in
    // batches.
    const MAX_BATCH_SIZE: usize = 5_000;

    let map = downloads
        .into_vec()
        .into_iter()
        .map(NewDownload::from)
        .collect::<Vec<_>>();

    for chunk in map.chunks(MAX_BATCH_SIZE) {
        diesel::insert_into(temp_downloads::table)
            .values(chunk)
            .execute(conn)?;
    }

    Ok(())
}

/// Saves the downloads from the temporary `temp_downloads` table to the
/// `version_downloads` table and returns the name/version combinations that
/// were not found in the database.
#[instrument(
    "db.query",
    skip_all,
    fields(message = "INSERT INTO version_downloads ...")
)]
fn save_to_version_downloads(conn: &mut impl Conn) -> QueryResult<Vec<NameAndVersion>> {
    diesel::sql_query(
        r#"
            WITH joined_data AS (
                SELECT versions.id, temp_downloads.*
                FROM temp_downloads
                LEFT JOIN crates ON crates.name = temp_downloads.name
                LEFT JOIN versions ON versions.num = temp_downloads.version AND versions.crate_id = crates.id
            ), inserted AS (
                INSERT INTO version_downloads (version_id, date, downloads)
                SELECT joined_data.id, joined_data.date, joined_data.downloads
                FROM joined_data
                WHERE joined_data.id IS NOT NULL
                ORDER BY joined_data.id, joined_data.date
                ON CONFLICT (version_id, date)
                DO UPDATE SET downloads = version_downloads.downloads + EXCLUDED.downloads
            )
            SELECT joined_data.name, joined_data.version
            FROM joined_data
            WHERE joined_data.id IS NULL;
        "#,
    )
        .load(conn)
}

table! {
    /// Imaginary table to make Diesel happy when using the `sql_query` macro in
    /// the [`save_to_version_downloads()`] function.
    name_and_versions (name, version) {
        name -> Text,
        version -> Text,
    }
}

/// A helper struct for the result of the query in the
/// [`save_to_version_downloads()`] function.
///
/// The result of `sql_query` can not be a tuple, so we have to define a
/// proper struct for the result.
#[derive(QueryableByName)]
struct NameAndVersion {
    name: String,
    version: String,
}

impl Debug for NameAndVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}@{}", self.name, self.version)
    }
}

/// Checks if the given log file has already been processed.
///
/// Acquires a connection from the pool before passing it to the
/// [`already_processed_inner()`] function.
async fn already_processed(
    path: impl Into<String>,
    db_pool: Pool<AsyncPgConnection>,
) -> anyhow::Result<bool> {
    let path = path.into();

    let conn = db_pool.get().await?;
    let already_processed = spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
        Ok::<_, anyhow::Error>(already_processed_inner(path, conn)?)
    })
    .await?;

    Ok(already_processed)
}

/// Checks if the given log file has already been processed by querying the
/// `processed_log_files` table for the given path.
///
/// Note that if a second job is already processing the same log file, this
/// function will return `false` because the second job will not have inserted
/// the path into the `processed_log_files` table yet.
fn already_processed_inner(path: impl Into<String>, conn: &mut impl Conn) -> QueryResult<bool> {
    use crate::schema::processed_log_files;

    let query = processed_log_files::table.filter(processed_log_files::path.eq(path.into()));
    select(exists(query)).get_result(conn)
}

/// Inserts the given path into the `processed_log_files` table to mark it as
/// processed.
fn save_as_processed(path: impl Into<String>, conn: &mut impl Conn) -> QueryResult<()> {
    use crate::schema::processed_log_files;

    diesel::insert_into(processed_log_files::table)
        .values(processed_log_files::path.eq(path.into()))
        .execute(conn)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{crates, version_downloads, versions};
    use crate::util::diesel::Conn;
    use crates_io_test_db::TestDatabase;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use insta::assert_debug_snapshot;

    const CLOUDFRONT_PATH: &str =
        "cloudfront/static.crates.io/E35K556QRQDZXW.2024-01-16-16.d01d5f13.gz";

    #[tokio::test]
    async fn test_process_cdn_log() {
        crate::util::tracing::init_for_test();

        let test_database = TestDatabase::new();
        let db_pool = build_connection_pool(test_database.url());
        create_dummy_crates_and_versions(db_pool.clone()).await;

        let store = build_dummy_store().await;

        assert_ok!({
            let store = store.clone();
            run(store, CLOUDFRONT_PATH, db_pool.clone()).await
        });
        assert_debug_snapshot!(all_version_downloads(db_pool.clone()).await, @r###"
        [
            "bindgen | 0.65.1 | 1 | 0 | 2024-01-16 | false",
            "quick-error | 1.2.3 | 2 | 0 | 2024-01-16 | false",
            "quick-error | 1.2.3 | 1 | 0 | 2024-01-17 | false",
            "tracing-core | 0.1.32 | 1 | 0 | 2024-01-16 | false",
        ]
        "###);

        // Check that processing the same log file again does not insert
        // duplicate data.
        assert_ok!(run(store, CLOUDFRONT_PATH, db_pool.clone()).await);
        assert_debug_snapshot!(all_version_downloads(db_pool).await, @r###"
        [
            "bindgen | 0.65.1 | 1 | 0 | 2024-01-16 | false",
            "quick-error | 1.2.3 | 2 | 0 | 2024-01-16 | false",
            "quick-error | 1.2.3 | 1 | 0 | 2024-01-17 | false",
            "tracing-core | 0.1.32 | 1 | 0 | 2024-01-16 | false",
        ]
        "###);
    }

    #[test]
    fn test_build_store_s3() {
        let access_key = "access_key".into();
        let secret_key = "secret_key".to_string().into();
        let config = CdnLogStorageConfig::s3(access_key, secret_key);
        assert_ok!(build_store(&config, "us-west-1", "bucket"));
    }

    #[test]
    fn test_build_store_local() {
        let path = std::env::current_dir().unwrap();
        let config = CdnLogStorageConfig::local(path);
        assert_ok!(build_store(&config, "us-west-1", "bucket"));
    }

    #[test]
    fn test_build_store_memory() {
        let config = CdnLogStorageConfig::memory();
        assert_ok!(build_store(&config, "us-west-1", "bucket"));
    }

    /// Builds a dummy object store with a log file in it.
    async fn build_dummy_store() -> Arc<dyn ObjectStore> {
        let store = InMemory::new();

        // Add dummy data to the store
        let path = CLOUDFRONT_PATH.into();
        let bytes = include_bytes!(
            "../../../../crates/crates_io_cdn_logs/test_data/cloudfront/basic.log.gz"
        );

        store.put(&path, bytes[..].into()).await.unwrap();

        Arc::new(store)
    }

    /// Builds a connection pool to the test database.
    fn build_connection_pool(url: &str) -> Pool<AsyncPgConnection> {
        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(url);
        Pool::builder(manager).build().unwrap()
    }

    /// Inserts some dummy crates and versions into the database.
    async fn create_dummy_crates_and_versions(db_pool: Pool<AsyncPgConnection>) {
        let conn = db_pool.get().await.unwrap();
        spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

            create_crate_and_version("bindgen", "0.65.1", conn);
            create_crate_and_version("tracing-core", "0.1.32", conn);
            create_crate_and_version("quick-error", "1.2.3", conn);

            Ok::<_, anyhow::Error>(())
        })
        .await
        .unwrap();
    }

    /// Inserts a dummy crate and version into the database.
    fn create_crate_and_version(name: &str, version: &str, conn: &mut impl Conn) {
        let crate_id: i32 = diesel::insert_into(crates::table)
            .values(crates::name.eq(name))
            .returning(crates::id)
            .get_result(conn)
            .unwrap();

        diesel::insert_into(versions::table)
            .values((
                versions::crate_id.eq(crate_id),
                versions::num.eq(version),
                versions::num_no_build.eq(version),
                versions::checksum.eq("checksum"),
            ))
            .execute(conn)
            .unwrap();
    }

    /// Queries all version downloads from the database and returns them as a
    /// [`Vec`] of strings for use with [`assert_debug_snapshot!()`].
    async fn all_version_downloads(db_pool: Pool<AsyncPgConnection>) -> Vec<String> {
        let conn = db_pool.get().await.unwrap();
        let downloads = spawn_blocking(move || {
            let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
            Ok::<_, anyhow::Error>(query_all_version_downloads(conn))
        })
        .await
        .unwrap();

        downloads
            .into_iter()
            .map(|(name, version, downloads, counted, date, processed)| {
                format!("{name} | {version} | {downloads} | {counted} | {date} | {processed}")
            })
            .collect()
    }

    /// Queries all version downloads from the database and returns them as a
    /// [`Vec`] of tuples.
    fn query_all_version_downloads(
        conn: &mut impl Conn,
    ) -> Vec<(String, String, i32, i32, NaiveDate, bool)> {
        version_downloads::table
            .inner_join(versions::table)
            .inner_join(crates::table.on(versions::crate_id.eq(crates::id)))
            .select((
                crates::name,
                versions::num,
                version_downloads::downloads,
                version_downloads::counted,
                version_downloads::date,
                version_downloads::processed,
            ))
            .order((crates::name, versions::num, version_downloads::date))
            .load(conn)
            .unwrap()
    }
}
