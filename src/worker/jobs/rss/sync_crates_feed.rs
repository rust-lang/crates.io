use crate::schema::crates;
use crate::storage::FeedId;
use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct SyncCratesFeed;

const NUM_ITEMS: i64 = 50;

impl BackgroundJob for SyncCratesFeed {
    const JOB_NAME: &'static str = "sync_crates_feed";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let feed_id = FeedId::Crates;
        let domain = &ctx.config.domain_name;

        info!("Loading latest {NUM_ITEMS} crates from the database…");
        let conn = ctx.deadpool.get().await?;
        let new_crates = conn
            .interact(load_new_crates)
            .await
            .map_err(|err| anyhow!(err.to_string()))??;

        let link = rss::extension::atom::Link {
            href: ctx.storage.feed_url(&feed_id),
            rel: "self".to_string(),
            mime_type: Some("application/rss+xml".to_string()),
            ..Default::default()
        };

        let items = new_crates
            .into_iter()
            .map(|c| c.into_rss_item(domain))
            .collect();

        let namespaces = vec![("crates".to_string(), "https://crates.io/".to_string())];
        let namespaces = namespaces.into_iter().collect();

        let channel = rss::Channel {
            title: "crates.io: newest crates".to_string(),
            link: format!("https://{domain}/"),
            description: "Newest crates registered on the crates.io package registry".to_string(),
            language: Some("en".to_string()),
            atom_ext: Some(rss::extension::atom::AtomExtension { links: vec![link] }),
            namespaces,
            items,
            ..Default::default()
        };

        info!("Uploading feed to storage…");
        ctx.storage.upload_feed(&feed_id, &channel).await?;

        let path = object_store::path::Path::from(&feed_id);
        if let Some(cloudfront) = ctx.cloudfront() {
            info!(%path, "Invalidating CloudFront cache…");
            cloudfront.invalidate(path.as_ref()).await?;
        } else {
            info!("Skipping CloudFront cache invalidation (CloudFront not configured)");
        }

        if let Some(fastly) = ctx.fastly() {
            info!(%path, "Invalidating Fastly cache…");
            fastly.invalidate(path.as_ref()).await?;
        } else {
            info!("Skipping Fastly cache invalidation (Fastly not configured)");
        }

        info!("Finished syncing crates feed");
        Ok(())
    }
}

fn load_new_crates(conn: &mut PgConnection) -> QueryResult<Vec<NewCrate>> {
    crates::table
        .order(crates::created_at.desc())
        .select(NewCrate::as_select())
        .limit(NUM_ITEMS)
        .load(conn)
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct NewCrate {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = crates::columns::description)]
    description: Option<String>,
    #[diesel(select_expression = crates::columns::created_at)]
    time: chrono::NaiveDateTime,
}

impl NewCrate {
    fn into_rss_item(self, domain: &str) -> rss::Item {
        let title = format!("New crate created: {}", self.name);
        let link = format!("https://{domain}/crates/{}", self.name);
        let pub_date = self.time.and_utc().to_rfc2822();

        let guid = rss::Guid {
            value: link.clone(),
            permalink: true,
        };

        let description = self
            .description
            .map(|d| quick_xml::escape::escape(&d).to_string());

        let name_extension = rss::extension::Extension {
            name: "crates:name".into(),
            value: Some(self.name),
            ..Default::default()
        };

        let extensions = vec![("name".to_string(), vec![name_extension])];
        let extensions = extensions.into_iter().collect();
        let extensions = vec![("crates".to_string(), extensions)];
        let extensions = extensions.into_iter().collect();

        rss::Item {
            guid: Some(guid),
            title: Some(title),
            link: Some(link),
            description,
            pub_date: Some(pub_date),
            extensions,
            ..Default::default()
        }
    }
}
