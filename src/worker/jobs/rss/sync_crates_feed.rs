use crate::schema::crates;
use crate::storage::FeedId;
use crate::worker::Environment;
use chrono::Duration;
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use std::sync::Arc;

#[derive(Serialize, Deserialize)]
pub struct SyncCratesFeed;

/// Items younger than this will always be included in the feed.
const ALWAYS_INCLUDE_AGE: Duration = Duration::minutes(60);

/// The number of items to include in the feed.
///
/// If there are less than this number of items in the database, the feed will
/// contain fewer items. If there are more items in the database that are
/// younger than [`ALWAYS_INCLUDE_AGE`], all of them will be included in
/// the feed.
const NUM_ITEMS: i64 = 50;

impl BackgroundJob for SyncCratesFeed {
    const JOB_NAME: &'static str = "sync_crates_feed";
    const DEDUPLICATED: bool = true;

    type Context = Arc<Environment>;

    async fn run(&self, ctx: Self::Context) -> anyhow::Result<()> {
        let feed_id = FeedId::Crates;
        let domain = &ctx.config.domain_name;

        info!("Loading latest {NUM_ITEMS} crates from the database…");
        let mut conn = ctx.deadpool.get().await?;
        let new_crates = load_new_crates(&mut conn).await?;

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
        if let Err(error) = ctx.invalidate_cdns(path.as_ref()).await {
            warn!("Failed to invalidate CDN caches: {error}");
        }

        info!("Finished syncing crates feed");
        Ok(())
    }
}

/// Load the latest crates from the database.
///
/// This function will load all crates from the database that are younger
/// than [`ALWAYS_INCLUDE_AGE`]. If there are less than [`NUM_ITEMS`] crates
/// then the list will be padded with older crates until [`NUM_ITEMS`] are
/// returned.
async fn load_new_crates(conn: &mut AsyncPgConnection) -> QueryResult<Vec<NewCrate>> {
    let threshold_dt = chrono::Utc::now().naive_utc() - ALWAYS_INCLUDE_AGE;

    let new_crates = crates::table
        .filter(crates::created_at.gt(threshold_dt))
        .order(crates::created_at.desc())
        .select(NewCrate::as_select())
        .load(conn)
        .await?;

    let num_new_crates = new_crates.len();
    if num_new_crates as i64 >= NUM_ITEMS {
        return Ok(new_crates);
    }

    crates::table
        .order(crates::created_at.desc())
        .select(NewCrate::as_select())
        .limit(NUM_ITEMS)
        .load(conn)
        .await
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
            description: self.description,
            pub_date: Some(pub_date),
            extensions,
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use crates_io_test_db::TestDatabase;
    use diesel_async::AsyncPgConnection;
    use futures_util::future::join_all;
    use insta::assert_debug_snapshot;
    use std::borrow::Cow;
    use std::future::Future;

    #[tokio::test]
    async fn test_load_version_updates() {
        crate::util::tracing::init_for_test();

        let db = TestDatabase::new();
        let mut conn = db.async_connect().await;

        let now = chrono::Utc::now().naive_utc();

        let new_crates = assert_ok!(load_new_crates(&mut conn).await);
        assert_eq!(new_crates.len(), 0);

        // If there are less than NUM_ITEMS crates, they should all be returned
        let futures = [
            create_crate(&mut conn, "foo", now - Duration::days(123)),
            create_crate(&mut conn, "bar", now - Duration::days(110)),
            create_crate(&mut conn, "baz", now - Duration::days(100)),
            create_crate(&mut conn, "qux", now - Duration::days(90)),
        ];
        join_all(futures).await;

        let new_crates = assert_ok!(load_new_crates(&mut conn).await);
        assert_eq!(new_crates.len(), 4);
        assert_debug_snapshot!(new_crates.iter().map(|u| &u.name).collect::<Vec<_>>());

        // If there are more than NUM_ITEMS crates, only the most recent NUM_ITEMS should be returned
        let mut futures = Vec::new();
        for i in 1..=NUM_ITEMS {
            let name = format!("crate-{i}");
            let publish_time = now - Duration::days(90) + Duration::hours(i);
            futures.push(create_crate(&mut conn, name, publish_time));
        }
        join_all(futures).await;

        let new_crates = assert_ok!(load_new_crates(&mut conn).await);
        assert_eq!(new_crates.len() as i64, NUM_ITEMS);
        assert_debug_snapshot!(new_crates.iter().map(|u| &u.name).collect::<Vec<_>>());

        // But if there are more than NUM_ITEMS crates that are younger than ALWAYS_INCLUDE_AGE, all of them should be returned
        let mut futures = Vec::new();
        for i in 1..=(NUM_ITEMS + 10) {
            let name = format!("other-crate-{i}");
            let publish_time = now - Duration::minutes(30) + Duration::seconds(i);
            futures.push(create_crate(&mut conn, name, publish_time));
        }
        join_all(futures).await;

        let new_crates = assert_ok!(load_new_crates(&mut conn).await);
        assert_eq!(new_crates.len() as i64, NUM_ITEMS + 10);
        assert_debug_snapshot!(new_crates.iter().map(|u| &u.name).collect::<Vec<_>>());
    }

    fn create_crate(
        conn: &mut AsyncPgConnection,
        name: impl Into<Cow<'static, str>>,
        publish_time: NaiveDateTime,
    ) -> impl Future<Output = i32> {
        let future = diesel::insert_into(crates::table)
            .values((
                crates::name.eq(name.into()),
                crates::created_at.eq(publish_time),
                crates::updated_at.eq(publish_time),
            ))
            .returning(crates::id)
            .get_result(conn);

        async move { future.await.unwrap() }
    }
}
