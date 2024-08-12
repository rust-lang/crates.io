use std::{collections::BTreeSet, sync::Arc};

use anyhow::Context;
use crates_io_worker::BackgroundJob;
use futures_util::TryStreamExt;
use object_store::{ObjectMeta, ObjectStore};

use crate::worker::Environment;

const INDEX_PATH: &str = "archive/version-downloads/index.html";

/// Generate an index.html for the version download CSVs exported to S3.
#[derive(Serialize, Deserialize, Default)]
pub struct IndexVersionDownloadsArchive;

impl BackgroundJob for IndexVersionDownloadsArchive {
    const JOB_NAME: &'static str = "index_version_downloads_archive";

    type Context = Arc<Environment>;

    async fn run(&self, env: Self::Context) -> anyhow::Result<()> {
        info!("Indexing old version downloads…");

        let Some(downloads_archive_store) = env.downloads_archive_store.as_ref() else {
            warn!("No downloads archive store configured");
            return Ok(());
        };

        info!("Generating and uploading index.html…");
        let files = match generate_html(downloads_archive_store).await {
            Ok(files) => files,
            Err(error) => {
                warn!("Error generating index.html: {error}");
                return Err(error);
            }
        };
        info!("index.html generated and uploaded");

        info!("Generating and uploading index.json…");
        if let Err(error) = generate_json(downloads_archive_store, files).await {
            warn!("Error generating index.json: {error}");
            return Err(error);
        }
        info!("index.json generated and uploaded");

        info!("Invalidating CDN caches…");
        if let Err(error) = env.invalidate_cdns(INDEX_PATH).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }
        info!("CDN caches invalidated");

        info!("Finished indexing old version downloads");
        Ok(())
    }
}

/// Generate and upload an index.html based on the objects within the given store.
async fn generate_html(store: &impl ObjectStore) -> anyhow::Result<FileSet> {
    let context = TemplateContext::new_from_store(store)
        .await
        .context("instantiating context from object store")?;
    let index = context.to_html().context("rendering template")?;

    store
        .put(&"index.html".into(), index.into())
        .await
        .context("uploading index.html")?;

    Ok(context.into())
}

/// Generate and upload an index.json based on the objects within the given store.
async fn generate_json(store: &impl ObjectStore, files: FileSet) -> anyhow::Result<()> {
    let content = serde_json::to_string(&files)?;

    store
        .put(&"index.json".into(), content.into())
        .await
        .context("uploading index.json")?;

    Ok(())
}

struct TemplateContext {
    env: minijinja::Environment<'static>,
    files: FileSet,
}

impl TemplateContext {
    pub async fn new_from_store(store: &impl ObjectStore) -> anyhow::Result<Self> {
        Ok(Self {
            env: Self::new_environment()?,
            files: FileSet::new_from_store(store).await?,
        })
    }

    pub fn to_html(&self) -> anyhow::Result<String> {
        use minijinja::context;

        Ok(self.env.get_template("index.html")?.render(context! {
            files => &self.files,
        })?)
    }

    fn new_environment() -> anyhow::Result<minijinja::Environment<'static>> {
        use minijinja::Environment;

        let mut env = Environment::new();
        env.add_template("index.html", include_str!("index.html.j2"))?;

        Ok(env)
    }
}

#[derive(Serialize, Debug, Default)]
struct FileSet(BTreeSet<File>);

impl FileSet {
    async fn new_from_store(store: &impl ObjectStore) -> anyhow::Result<Self> {
        let mut set = BTreeSet::new();
        let mut contents = store.list(None);
        while let Some(object) = contents.try_next().await? {
            match File::try_from(object) {
                Ok(file) => {
                    set.insert(file);
                }
                Err(e) => {
                    warn!(?e, "ignoring file in object store");
                }
            }
        }

        Ok(Self(set))
    }
}

impl From<TemplateContext> for FileSet {
    fn from(context: TemplateContext) -> Self {
        context.files
    }
}

#[derive(Serialize, Debug, Eq)]
struct File {
    name: String,
    size: usize,
}

impl Ord for File {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // This is intentionally reversed so that the most recent file appears at the top of the
        // index.
        other.name.cmp(&self.name)
    }
}

impl PartialEq for File {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl PartialOrd for File {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl TryFrom<ObjectMeta> for File {
    type Error = anyhow::Error;

    fn try_from(object: ObjectMeta) -> Result<Self, Self::Error> {
        match object.location.filename() {
            Some(filename) if filename.ends_with(".csv") => Ok(Self {
                name: filename.to_string(),
                size: object.size,
            }),
            Some(filename) => Err(anyhow::anyhow!("ignoring non-CSV file: {filename}")),
            None => Err(anyhow::anyhow!(
                "cannot get filename for object: {object:?}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use googletest::prelude::*;
    use insta::assert_snapshot;
    use object_store::memory::InMemory;

    use super::*;

    #[tokio::test]
    async fn test_generate_html() -> anyhow::Result<()> {
        let store = fake_store().await?;
        generate_html(&store).await?;

        let index = store.get(&"index.html".into()).await?.bytes().await?;

        // This should have overwritten the previous file of just null bytes.
        assert_that!(index.iter().any(|b| *b != b'\0'), eq(true));

        assert_snapshot!(std::str::from_utf8(&index)?);

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_html_empty() -> anyhow::Result<()> {
        let store = InMemory::new();
        generate_html(&store).await?;

        let index = store.get(&"index.html".into()).await?.bytes().await?;
        assert_snapshot!(std::str::from_utf8(&index)?);

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_json() -> anyhow::Result<()> {
        let store = fake_store().await?;
        let context = TemplateContext::new_from_store(&store).await?;
        generate_json(&store, context.into()).await?;

        let index = store.get(&"index.json".into()).await?.bytes().await?;
        assert_snapshot!(std::str::from_utf8(&index)?);

        Ok(())
    }

    #[tokio::test]
    async fn test_generate_json_empty() -> anyhow::Result<()> {
        let store = fake_store().await?;
        let context = TemplateContext::new_from_store(&store).await?;
        generate_json(&store, context.into()).await?;

        let index = store.get(&"index.json".into()).await?.bytes().await?;
        assert_snapshot!(std::str::from_utf8(&index)?);

        Ok(())
    }

    #[tokio::test]
    async fn test_template_context() -> anyhow::Result<()> {
        let store = fake_store().await?;
        let context = TemplateContext::new_from_store(&store).await?;

        // Validate that only the expected date CSVs are present, in order.
        let filenames: Vec<_> = context
            .files
            .0
            .iter()
            .map(|file| file.name.as_str())
            .collect();

        assert_that!(
            filenames,
            container_eq([
                "2024-08-01.csv",
                "2024-07-31.csv",
                "2024-07-30.csv",
                "2024-07-29.csv"
            ]),
        );

        assert_snapshot!(context.to_html()?);

        Ok(())
    }

    async fn fake_store() -> anyhow::Result<InMemory> {
        let store = InMemory::new();

        for (name, size) in [
            // Firstly, here are some plausible fake entries in random order.
            ("2024-07-31.csv", 123),
            ("2024-07-30.csv", 124),
            ("2024-08-01.csv", 138),
            ("2024-07-29.csv", 234),
            // Now for the junk that we want to ignore. Let's put in an index.
            ("index.html", 40),
            // And a nested file that isn't CSV at all.
            ("foo/bar", 50),
        ] {
            store.put(&name.into(), vec![0u8; size].into()).await?;
        }

        Ok(store)
    }
}
