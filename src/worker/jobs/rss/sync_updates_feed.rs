use crate::schema::{crates, versions};
use crate::storage::FeedId;
use crate::worker::Environment;
use anyhow::anyhow;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct SyncUpdatesFeed;

const NUM_ITEMS: i64 = 100;

impl BackgroundJob for SyncUpdatesFeed {
    const JOB_NAME: &'static str = "sync_updates_feed";

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let feed_id = FeedId::Updates;
        let domain = &ctx.config.domain_name;

        info!("Loading latest {NUM_ITEMS} version updates from the database…");
        let conn = ctx.deadpool.get().await?;
        let version_updates = conn
            .interact(load_version_updates)
            .await
            .map_err(|err| anyhow!(err.to_string()))??;

        let link = rss::extension::atom::Link {
            href: ctx.storage.feed_url(&feed_id),
            rel: "self".to_string(),
            mime_type: Some("application/rss+xml".to_string()),
            ..Default::default()
        };

        let items = version_updates
            .into_iter()
            .map(|u| u.into_rss_item(domain))
            .collect();

        let channel = rss::Channel {
            title: "crates.io: recent updates".to_string(),
            link: format!("https://{domain}/"),
            description: "Recent version publishes on the crates.io package registry".to_string(),
            language: Some("en".to_string()),
            atom_ext: Some(rss::extension::atom::AtomExtension { links: vec![link] }),
            items,
            ..Default::default()
        };

        info!("Uploading feed to storage…");
        ctx.storage.upload_feed(&feed_id, &channel).await?;

        if let Some(cloudfront) = ctx.cloudfront() {
            let path = object_store::path::Path::from(&feed_id);

            info!(%path, "Invalidating CloudFront cache…");
            cloudfront.invalidate(path.as_ref()).await?;
        } else {
            info!("Skipping CloudFront cache invalidation (CloudFront not configured)");
        }

        info!("Finished syncing updates feed");
        Ok(())
    }
}

fn load_version_updates(conn: &mut PgConnection) -> QueryResult<Vec<VersionUpdate>> {
    versions::table
        .inner_join(crates::table)
        .order(versions::created_at.desc())
        .select(VersionUpdate::as_select())
        .limit(NUM_ITEMS)
        .load(conn)
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct VersionUpdate {
    #[diesel(select_expression = crates::columns::name)]
    name: String,
    #[diesel(select_expression = versions::columns::num)]
    version: String,
    #[diesel(select_expression = crates::columns::description)]
    description: Option<String>,
    #[diesel(select_expression = versions::columns::created_at)]
    time: chrono::NaiveDateTime,
}

impl VersionUpdate {
    fn into_rss_item(self, domain: &str) -> rss::Item {
        let title = format!(
            "New crate version published: {} v{}",
            self.name, self.version
        );
        let link = format!("https://{domain}/crates/{}/{}", self.name, self.version);
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

        let version_extension = rss::extension::Extension {
            name: "crates:version".into(),
            value: Some(self.version),
            ..Default::default()
        };

        let extensions = vec![
            ("name".to_string(), vec![name_extension]),
            ("version".to_string(), vec![version_extension]),
        ];
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
