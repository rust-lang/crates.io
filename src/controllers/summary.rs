use crate::app::AppState;
use crate::models::{Category, Crate, CrateVersions, Keyword, TopVersions, Version};
use crate::schema::{crate_downloads, crates, keywords, metadata, recent_crate_downloads};
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::AppResult;
use crate::views::{EncodableCategory, EncodableCrate, EncodableKeyword};
use axum::Json;
use diesel::prelude::*;
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use serde_json::Value;

/// Handles the `GET /summary` route.
pub async fn summary(state: AppState) -> AppResult<Json<Value>> {
    let conn = state.db_read().await?;
    spawn_blocking(move || {
        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let config = &state.config;

        let num_crates: i64 = crates::table.count().get_result(conn)?;
        let num_downloads: i64 = metadata::table
            .select(metadata::total_downloads)
            .get_result(conn)?;

        fn encode_crates(
            conn: &mut impl Conn,
            data: Vec<(Crate, i64, Option<i64>)>,
        ) -> AppResult<Vec<EncodableCrate>> {
            let downloads = data
                .iter()
                .map(|&(_, total, recent)| (total, recent))
                .collect::<Vec<_>>();

            let krates = data.into_iter().map(|(c, _, _)| c).collect::<Vec<_>>();

            let versions: Vec<Version> = krates.versions().load(conn)?;
            versions
                .grouped_by(&krates)
                .into_iter()
                .map(TopVersions::from_versions)
                .zip(krates)
                .zip(downloads)
                .map(|((top_versions, krate), (total, recent))| {
                    Ok(EncodableCrate::from_minimal(
                        krate,
                        Some(&top_versions),
                        None,
                        false,
                        total,
                        recent,
                    ))
                })
                .collect()
        }

        let selection = (
            Crate::as_select(),
            crate_downloads::downloads,
            recent_crate_downloads::downloads.nullable(),
        );

        let new_crates = crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .order(crates::created_at.desc())
            .select(selection)
            .limit(10)
            .load(conn)?;
        let just_updated = crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .filter(crates::updated_at.ne(crates::created_at))
            .order(crates::updated_at.desc())
            .select(selection)
            .limit(10)
            .load(conn)?;

        let mut most_downloaded_query = crates::table
            .inner_join(crate_downloads::table)
            .left_join(recent_crate_downloads::table)
            .into_boxed();
        if !config.excluded_crate_names.is_empty() {
            most_downloaded_query =
                most_downloaded_query.filter(crates::name.ne_all(&config.excluded_crate_names));
        }
        let most_downloaded = most_downloaded_query
            .then_order_by(crate_downloads::downloads.desc())
            .select(selection)
            .limit(10)
            .load(conn)?;

        let mut most_recently_downloaded_query = crates::table
            .inner_join(crate_downloads::table)
            .inner_join(recent_crate_downloads::table)
            .into_boxed();
        if !config.excluded_crate_names.is_empty() {
            most_recently_downloaded_query = most_recently_downloaded_query
                .filter(crates::name.ne_all(&config.excluded_crate_names));
        }
        let most_recently_downloaded = most_recently_downloaded_query
            .then_order_by(recent_crate_downloads::downloads.desc())
            .select(selection)
            .limit(10)
            .load(conn)?;

        let popular_keywords = keywords::table
            .order(keywords::crates_cnt.desc())
            .limit(10)
            .load(conn)?
            .into_iter()
            .map(Keyword::into)
            .collect::<Vec<EncodableKeyword>>();

        let popular_categories = Category::toplevel(conn, "crates", 10, 0)?
            .into_iter()
            .map(Category::into)
            .collect::<Vec<EncodableCategory>>();

        Ok(Json(json!({
            "num_downloads": num_downloads,
            "num_crates": num_crates,
            "new_crates": encode_crates(conn, new_crates)?,
            "most_downloaded": encode_crates(conn, most_downloaded)?,
            "most_recently_downloaded": encode_crates(conn, most_recently_downloaded)?,
            "just_updated": encode_crates(conn, just_updated)?,
            "popular_keywords": popular_keywords,
            "popular_categories": popular_categories,
        })))
    })
    .await
}
