use crate::app::AppState;
use crate::models::{Category, Crate, Keyword, TopVersions, Version};
use crate::schema::{
    crate_downloads, crates, default_versions, keywords, metadata, recent_crate_downloads, versions,
};
use crate::util::errors::AppResult;
use crate::views::{EncodableCategory, EncodableCrate, EncodableKeyword};
use axum_extra::json;
use axum_extra::response::ErasedJson;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use futures_util::FutureExt;
use std::future::Future;

/// Handles the `GET /summary` route.
pub async fn summary(state: AppState) -> AppResult<ErasedJson> {
    let mut conn = state.db_read().await?;

    let config = &state.config;

    let (
        num_crates,
        num_downloads,
        new_crates,
        just_updated,
        most_downloaded,
        most_recently_downloaded,
        popular_categories,
        popular_keywords,
    ) = tokio::try_join!(
        crates::table.count().get_result::<i64>(&mut conn).boxed(),
        metadata::table
            .select(metadata::total_downloads)
            .get_result::<i64>(&mut conn)
            .boxed(),
        crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .left_join(default_versions::table)
            .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
            .order(crates::created_at.desc())
            .select(Record::as_select())
            .limit(10)
            .load(&mut conn)
            .boxed(),
        crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .left_join(default_versions::table)
            .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
            .filter(crates::updated_at.ne(crates::created_at))
            .order(crates::updated_at.desc())
            .select(Record::as_select())
            .limit(10)
            .load(&mut conn)
            .boxed(),
        crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .left_join(default_versions::table)
            .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
            .filter(crates::name.ne_all(&config.excluded_crate_names))
            .then_order_by(crate_downloads::downloads.desc())
            .select(Record::as_select())
            .limit(10)
            .load(&mut conn)
            .boxed(),
        crates::table
            .inner_join(crate_downloads::table)
            .inner_join(recent_crate_downloads::table)
            .left_join(default_versions::table)
            .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
            .filter(crates::name.ne_all(&config.excluded_crate_names))
            .then_order_by(recent_crate_downloads::downloads.desc())
            .select(Record::as_select())
            .limit(10)
            .load(&mut conn)
            .boxed(),
        Category::toplevel(&mut conn, "crates", 10, 0),
        keywords::table
            .order(keywords::crates_cnt.desc())
            .limit(10)
            .load(&mut conn)
            .boxed(),
    )?;

    let (new_crates, most_downloaded, most_recently_downloaded, just_updated) = tokio::try_join!(
        encode_crates(&mut conn, new_crates),
        encode_crates(&mut conn, most_downloaded),
        encode_crates(&mut conn, most_recently_downloaded),
        encode_crates(&mut conn, just_updated),
    )?;

    let popular_categories = popular_categories
        .into_iter()
        .map(Category::into)
        .collect::<Vec<EncodableCategory>>();

    let popular_keywords = popular_keywords
        .into_iter()
        .map(Keyword::into)
        .collect::<Vec<EncodableKeyword>>();

    Ok(json!({
        "num_downloads": num_downloads,
        "num_crates": num_crates,
        "new_crates": new_crates,
        "most_downloaded": most_downloaded,
        "most_recently_downloaded": most_recently_downloaded,
        "just_updated": just_updated,
        "popular_keywords": popular_keywords,
        "popular_categories": popular_categories,
    }))
}

#[derive(Debug, Queryable, Selectable)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Record {
    #[diesel(embed)]
    krate: Crate,
    #[diesel(select_expression = crate_downloads::columns::downloads)]
    total_downloads: i64,
    #[diesel(select_expression = recent_crate_downloads::columns::downloads.nullable())]
    recent_downloads: Option<i64>,
    #[diesel(select_expression = versions::columns::num.nullable())]
    default_version: Option<String>,
    #[diesel(select_expression = versions::columns::yanked.nullable())]
    yanked: Option<bool>,
}

fn encode_crates(
    conn: &mut AsyncPgConnection,
    data: Vec<Record>,
) -> impl Future<Output = AppResult<Vec<EncodableCrate>>> {
    let crate_ids = data
        .iter()
        .map(|record| record.krate.id)
        .collect::<Vec<_>>();

    let future = versions::table
        .filter(versions::crate_id.eq_any(crate_ids))
        .filter(versions::yanked.eq(false))
        .select(Version::as_select())
        .load(conn);

    async move {
        let versions: Vec<Version> = future.await?;

        let krates = data.iter().map(|record| &record.krate).collect::<Vec<_>>();
        versions
            .grouped_by(&krates)
            .into_iter()
            .map(TopVersions::from_versions)
            .zip(data)
            .map(|(top_versions, record)| {
                Ok(EncodableCrate::from_minimal(
                    record.krate,
                    record.default_version.as_deref(),
                    record.yanked,
                    Some(&top_versions),
                    false,
                    record.total_downloads,
                    record.recent_downloads,
                ))
            })
            .collect()
    }
}
