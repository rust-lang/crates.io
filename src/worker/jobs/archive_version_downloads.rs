use crate::schema::version_downloads;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::worker::Environment;
use anyhow::{anyhow, Context};
use chrono::{NaiveDate, Utc};
use crates_io_env_vars::var;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel::{ExpressionMethods, RunQueryDsl};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use diesel_async::pooled_connection::deadpool::Pool;
use diesel_async::AsyncPgConnection;
use futures_util::StreamExt;
use object_store::aws::AmazonS3Builder;
use object_store::ObjectStore;
use secrecy::{ExposeSecret, SecretString};
use std::collections::btree_map::Entry;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;
use tempfile::tempdir;

const FILE_NAME: &str = "version_downloads.csv";

/// Archive data from the `version_downloads` table older than the given
/// date to S3.
///
/// This job first exports the data from the database to a CSV file using `psql`
/// and a `COPY` command. The CSV file is then split into multiple files based
/// on the date column and those are uploaded to the object store. Finally, the
/// successfully uploaded dates are deleted from the database.
#[derive(Serialize, Deserialize)]
pub struct ArchiveVersionDownloads {
    before: NaiveDate,
}

impl ArchiveVersionDownloads {
    pub fn before(before: NaiveDate) -> Self {
        Self { before }
    }

    pub fn store_from_environment() -> anyhow::Result<Option<Box<dyn ObjectStore>>> {
        let Some(region) = var("DOWNLOADS_ARCHIVE_REGION")? else {
            return Ok(None);
        };
        let Some(bucket) = var("DOWNLOADS_ARCHIVE_BUCKET")? else {
            return Ok(None);
        };
        let Some(access_key) = var("DOWNLOADS_ARCHIVE_ACCESS_KEY")? else {
            return Ok(None);
        };
        let Some(secret_key) = var("DOWNLOADS_ARCHIVE_SECRET_KEY")? else {
            return Ok(None);
        };

        let store = AmazonS3Builder::new()
            .with_region(region)
            .with_bucket_name(bucket)
            .with_access_key_id(access_key)
            .with_secret_access_key(secret_key)
            .build()
            .context("Failed to initialize S3 code")?;

        Ok(Some(Box::new(store)))
    }
}

impl Default for ArchiveVersionDownloads {
    fn default() -> Self {
        Self::before(Utc::now().date_naive() - chrono::Duration::days(90))
    }
}

impl BackgroundJob for ArchiveVersionDownloads {
    const JOB_NAME: &'static str = "archive_version_downloads";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Archiving old version downloads…");

        let Some(downloads_archive_store) = env.downloads_archive_store.as_ref() else {
            warn!("No downloads archive store configured");
            return Ok(());
        };

        let tempdir = tempdir().context("Failed to create temporary directory")?;
        let csv_path = tempdir.path().join(FILE_NAME);

        export(&env.config.db.primary.url, &csv_path, &self.before).await?;
        let dates = spawn_blocking(move || split(csv_path)).await?;
        let uploaded_dates = upload(downloads_archive_store, tempdir.path(), dates).await?;
        delete(&env.deadpool, uploaded_dates).await?;

        info!("Finished archiving old version downloads");
        Ok(())
    }
}

/// Export a subset of the `version_downloads` table to a CSV file.
///
/// The subset includes all rows with a date before the given `before` date.
async fn export(
    database_url: &SecretString,
    filename: impl AsRef<Path>,
    before: &NaiveDate,
) -> anyhow::Result<()> {
    let filename = filename.as_ref().as_os_str();
    let filename = filename
        .to_str()
        .ok_or_else(|| anyhow!("Invalid filename"))?;

    info!("Exporting version downloads to CSV file…");
    let instant = Instant::now();
    let command = format!("\\copy (SELECT date, version_id, downloads FROM version_downloads WHERE date < '{before}') TO '{filename}' WITH CSV HEADER");
    psql(database_url, &command).await?;

    let elapsed = instant.elapsed();
    info!("Finished exporting version downloads to CSV file ({elapsed:?})");

    Ok(())
}

/// Run a psql command on the given database.
///
/// Returns an error with the stderr output if the command fails.
async fn psql(database_url: &SecretString, command: &str) -> anyhow::Result<()> {
    debug!(?command, "Running psql script…");
    let output = tokio::process::Command::new("psql")
        .arg(database_url.expose_secret())
        .arg("-c")
        .arg(command)
        .output()
        .await
        .context("Failed to run psql command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("Failed to run psql command: {}", stderr));
    }

    Ok(())
}

/// Split the version downloads CSV file into multiple files.
///
/// The file is split based on the value of the first column, which is assumed
/// to be the `date` and dropped from the resulting files. The date is used as
/// the filename for the new CSV files, which are created in the same directory
/// as the original file.
fn split(path: impl AsRef<Path>) -> anyhow::Result<Vec<NaiveDate>> {
    let path = path.as_ref();

    info!(path = %path.display(), "Splitting CSV file into multiple files…");

    let instant = Instant::now();
    let parent_path = path.parent().ok_or_else(|| anyhow!("Invalid path"))?;

    let mut reader = csv::Reader::from_path(path)?;
    let mut writers: BTreeMap<Vec<u8>, _> = BTreeMap::new();

    let headers = reader.byte_headers()?.clone();
    let mut row = csv::ByteRecord::new();
    while reader.read_byte_record(&mut row)? {
        let mut row_iter = row.iter();
        let date = row_iter.next();
        let date = date.ok_or_else(|| anyhow!("Missing first column"))?;

        let mut entry = writers.entry(date.to_vec());
        let (_, writer) = match entry {
            Entry::Occupied(ref mut occupied) => occupied.get_mut(),
            Entry::Vacant(vacant) => {
                let date = std::str::from_utf8(date)?;
                let date = NaiveDate::parse_from_str(date, "%Y-%m-%d")?;

                let path = parent_path.join(format!("{date}.csv"));

                debug!(path = %path.display(), "Creating new CSV file for {date}…");
                let mut writer = csv::Writer::from_path(path)?;

                writer.write_record(headers.iter().skip(1))?;

                vacant.insert((date, writer))
            }
        };

        writer.write_record(row_iter)?;
    }

    let elapsed = instant.elapsed();
    info!("Finished splitting CSV file into multiple files ({elapsed:?})");

    Ok(writers.into_values().map(|(date, _)| date).collect())
}

/// Upload per-date CSV files from the given directory to the object store.
///
/// Uploads are done concurrently with a maximum of 10 files at a time and
/// only the dates for which the upload was successful are returned. For
/// failed uploads, a warning is logged.
async fn upload(
    store: &impl ObjectStore,
    directory: impl AsRef<Path>,
    dates: Vec<NaiveDate>,
) -> anyhow::Result<Vec<NaiveDate>> {
    // Upload at most 10 files concurrently.
    const MAX_CONCURRENCY: usize = 10;

    let directory = directory.as_ref();
    let uploaded_dates = futures_util::stream::iter(dates)
        .map(|date| async move {
            let path = directory.join(format!("{date}.csv"));
            let result = upload_file(store, &path).await;
            result.map(|_| date).inspect_err(|error| {
                warn!(path = %path.display(), "Failed to upload file to S3: {error}");
            })
        })
        .buffer_unordered(MAX_CONCURRENCY)
        .filter_map(|result| async { result.ok() })
        .collect::<Vec<_>>()
        .await;

    Ok(uploaded_dates)
}

/// Upload a single file to the object store.
async fn upload_file(store: &impl ObjectStore, path: impl AsRef<Path>) -> anyhow::Result<()> {
    let path = path.as_ref();
    let content = tokio::fs::read(path).await?;

    let filename = path
        .file_name()
        .and_then(|filename| filename.to_str())
        .ok_or_else(|| anyhow!("Invalid path"))?;

    let path = object_store::path::Path::parse(filename)?;

    debug!(%path, "Uploading file to S3…");
    store.put(&path, content.into()).await?;

    Ok(())
}

/// Delete version downloads for the given dates from the database.
async fn delete(db_pool: &Pool<AsyncPgConnection>, dates: Vec<NaiveDate>) -> anyhow::Result<()> {
    let conn = db_pool.get().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();
        delete_inner(conn, dates)
    })
    .await
}

fn delete_inner(conn: &mut impl Conn, dates: Vec<NaiveDate>) -> anyhow::Result<()> {
    // Delete version downloads for the given dates in chunks to avoid running
    // into the maximum query parameter limit.
    const CHUNK_SIZE: usize = 5000;

    info!("Deleting old version downloads for {} dates…", dates.len());
    for chunk in dates.chunks(CHUNK_SIZE) {
        let subset = version_downloads::table.filter(version_downloads::date.eq_any(chunk));
        match diesel::delete(subset).execute(conn) {
            Ok(num_deleted_rows) => {
                info!("Deleted {num_deleted_rows} rows from `version_downloads`");
            }
            Err(err) => {
                error!("Failed to delete rows from `version_downloads`: {err}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::{crates, version_downloads, versions};
    use crates_io_test_db::TestDatabase;
    use diesel_async::pooled_connection::AsyncDieselConnectionManager;
    use insta::assert_snapshot;

    #[tokio::test]
    async fn test_export() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.connect();
        prepare_database(&mut conn);

        let tempdir = tempdir().unwrap();
        let csv_path = tempdir.path().join(FILE_NAME);

        let database_url = SecretString::from(test_db.url().to_string());
        let before = NaiveDate::from_ymd_opt(2021, 1, 3).unwrap();
        export(&database_url, &csv_path, &before).await.unwrap();

        let content = tokio::fs::read_to_string(&csv_path).await.unwrap();
        assert_snapshot!(content, @r###"
        date,version_id,downloads
        2021-01-01,1,100
        2021-01-02,1,200
        2021-01-01,2,400
        2021-01-02,2,500
        "###);
    }

    #[test]
    fn test_split() {
        let tempdir = tempdir().unwrap();
        let csv_path = tempdir.path().join(FILE_NAME);
        std::fs::write(
            &csv_path,
            r###"
            date,version_id,downloads
            2021-01-01,1,100
            2021-01-02,1,200
            2021-01-03,1,300
            2021-01-01,2,400
            2021-01-02,2,500
            2021-01-03,2,600
            "###
            .trim(),
        )
        .unwrap();

        let dates = split(&csv_path).unwrap();
        let dates = dates
            .into_iter()
            .map(|date| date.to_string())
            .collect::<Vec<_>>();

        assert_eq!(dates, vec!["2021-01-01", "2021-01-02", "2021-01-03"]);

        let csv_path = tempdir.path().join("2021-01-02.csv");
        let content = std::fs::read_to_string(csv_path).unwrap();
        assert_snapshot!(content, @r###"
        version_id,downloads
        1,200
        2,500
        "###);
    }

    #[tokio::test]
    async fn test_upload() {
        let tempdir = tempdir().unwrap();
        let dir_path = tempdir.path();

        let csv_path = dir_path.join("2021-01-01.csv");
        let content = "version_id,downloads\n1,100\n2,400";
        std::fs::write(&csv_path, content).unwrap();

        let csv_path = dir_path.join("2021-01-02.csv");
        let content = "version_id,downloads\n1,200\n2,500";
        std::fs::write(&csv_path, content).unwrap();

        let store = object_store::memory::InMemory::new();
        let dates = vec![
            NaiveDate::from_ymd_opt(2021, 1, 1).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 2).unwrap(),
            NaiveDate::from_ymd_opt(2021, 1, 3).unwrap(),
        ];
        upload(&store, &dir_path, dates).await.unwrap();

        let store_path = object_store::path::Path::from("2021-01-01.csv");
        let result = store.get(&store_path).await.unwrap();
        let bytes = result.bytes().await.unwrap();
        assert_snapshot!(std::str::from_utf8(&bytes).unwrap(), @r###"
        version_id,downloads
        1,100
        2,400
        "###);

        let store_path = object_store::path::Path::from("2021-01-02.csv");
        let result = store.get(&store_path).await.unwrap();
        let bytes = result.bytes().await.unwrap();
        assert_snapshot!(std::str::from_utf8(&bytes).unwrap(), @r###"
        version_id,downloads
        1,200
        2,500
        "###);

        let store_path = object_store::path::Path::from("2021-01-03.csv");
        assert_err!(store.get(&store_path).await);
    }

    #[tokio::test]
    async fn test_delete() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.connect();
        prepare_database(&mut conn);

        let manager = AsyncDieselConnectionManager::<AsyncPgConnection>::new(test_db.url());
        let db_pool = Pool::builder(manager).build().unwrap();
        let dates = vec![NaiveDate::from_ymd_opt(2021, 1, 1).unwrap()];
        delete(&db_pool, dates).await.unwrap();

        let row_count: i64 = version_downloads::table
            .count()
            .get_result(&mut conn)
            .unwrap();
        assert_eq!(row_count, 4);
    }

    fn prepare_database(conn: &mut impl Conn) {
        let c1 = create_crate(conn, "foo");
        let v1 = create_version(conn, c1, "1.0.0");
        let v2 = create_version(conn, c1, "2.0.0");
        insert_downloads(conn, v1, "2021-01-01", 100);
        insert_downloads(conn, v1, "2021-01-02", 200);
        insert_downloads(conn, v1, "2021-01-03", 300);
        insert_downloads(conn, v2, "2021-01-01", 400);
        insert_downloads(conn, v2, "2021-01-02", 500);
        insert_downloads(conn, v2, "2021-01-03", 600);
    }

    fn create_crate(conn: &mut impl Conn, name: &str) -> i32 {
        diesel::insert_into(crates::table)
            .values(crates::name.eq(name))
            .returning(crates::id)
            .get_result(conn)
            .unwrap()
    }

    fn create_version(conn: &mut impl Conn, crate_id: i32, num: &str) -> i32 {
        diesel::insert_into(versions::table)
            .values((
                versions::crate_id.eq(crate_id),
                versions::num.eq(num),
                versions::checksum.eq(""),
            ))
            .returning(versions::id)
            .get_result(conn)
            .unwrap()
    }

    fn insert_downloads(conn: &mut impl Conn, version_id: i32, date: &str, downloads: i32) {
        let date = NaiveDate::parse_from_str(date, "%Y-%m-%d").unwrap();

        diesel::insert_into(version_downloads::table)
            .values((
                version_downloads::version_id.eq(version_id),
                version_downloads::date.eq(date),
                version_downloads::downloads.eq(downloads),
            ))
            .execute(conn)
            .unwrap();
    }
}
