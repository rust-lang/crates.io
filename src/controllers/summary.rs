use crate::app::AppState;
use crate::models::{Category, Crate, Keyword, TopVersions, Version};
use crate::schema::{
    crate_downloads, crates, default_versions, keywords, metadata, recent_crate_downloads, versions,
};
use crate::util::errors::AppResult;
use crate::views::{EncodableCategory, EncodableCrate, EncodableKeyword};
use axum::Json;
use diesel::{
    BelongingToDsl, ExpressionMethods, JoinOnDsl, NullableExpressionMethods, QueryDsl,
    SelectableHelper,
};
use diesel_async::AsyncPgConnection;
use diesel_async::RunQueryDsl;
use serde_json::Value;

/// Handles the `GET /summary` route.
pub async fn summary(state: AppState) -> AppResult<Json<Value>> {
    let mut conn = state.db_read().await?;

    let popular_categories = Category::toplevel(&mut conn, "crates", 10, 0)
        .await?
        .into_iter()
        .map(Category::into)
        .collect::<Vec<EncodableCategory>>();

    let num_crates: i64 = crates::table.count().get_result(&mut conn).await?;
    let num_downloads: i64 = metadata::table
        .select(metadata::total_downloads)
        .get_result(&mut conn)
        .await?;

    async fn encode_crates(
        conn: &mut AsyncPgConnection,
        data: Vec<Record>,
    ) -> AppResult<Vec<EncodableCrate>> {
        use diesel::GroupedBy;
        use diesel_async::RunQueryDsl;

        let krates = data.iter().map(|(c, ..)| c).collect::<Vec<_>>();
        let versions: Vec<Version> = Version::belonging_to(&krates)
            .filter(versions::yanked.eq(false))
            .load(conn)
            .await?;

        versions
            .grouped_by(&krates)
            .into_iter()
            .map(TopVersions::from_versions)
            .zip(data)
            .map(
                |(top_versions, (krate, total, recent, default_version, yanked))| {
                    Ok(EncodableCrate::from_minimal(
                        krate,
                        default_version.as_deref(),
                        yanked,
                        Some(&top_versions),
                        false,
                        total,
                        recent,
                    ))
                },
            )
            .collect()
    }

    let config = &state.config;

    let selection = (
        Crate::as_select(),
        crate_downloads::downloads,
        recent_crate_downloads::downloads.nullable(),
        versions::num.nullable(),
        versions::yanked.nullable(),
    );

    let new_crates = crates::table
        .inner_join(crate_downloads::table)
        .left_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .order(crates::created_at.desc())
        .select(selection)
        .limit(10)
        .load(&mut conn)
        .await?;
    let just_updated = crates::table
        .inner_join(crate_downloads::table)
        .left_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .filter(crates::updated_at.ne(crates::created_at))
        .order(crates::updated_at.desc())
        .select(selection)
        .limit(10)
        .load(&mut conn)
        .await?;

    let most_downloaded = crates::table
        .inner_join(crate_downloads::table)
        .left_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .filter(crates::name.ne_all(&config.excluded_crate_names))
        .then_order_by(crate_downloads::downloads.desc())
        .select(selection)
        .limit(10)
        .load(&mut conn)
        .await?;

    let most_recently_downloaded = crates::table
        .inner_join(crate_downloads::table)
        .inner_join(recent_crate_downloads::table)
        .left_join(default_versions::table)
        .left_join(versions::table.on(default_versions::version_id.eq(versions::id)))
        .filter(crates::name.ne_all(&config.excluded_crate_names))
        .then_order_by(recent_crate_downloads::downloads.desc())
        .select(selection)
        .limit(10)
        .load(&mut conn)
        .await?;

    let popular_keywords = keywords::table
        .order(keywords::crates_cnt.desc())
        .limit(10)
        .load(&mut conn)
        .await?
        .into_iter()
        .map(Keyword::into)
        .collect::<Vec<EncodableKeyword>>();

    Ok(Json(json!({
        "num_downloads": num_downloads,
        "num_crates": num_crates,
        "new_crates": encode_crates(&mut conn, new_crates).await?,
        "most_downloaded": encode_crates(&mut conn, most_downloaded).await?,
        "most_recently_downloaded": encode_crates(&mut conn, most_recently_downloaded).await?,
        "just_updated": encode_crates(&mut conn, just_updated).await?,
        "popular_keywords": popular_keywords,
        "popular_categories": popular_categories,
    })))
}

type Record = (Crate, i64, Option<i64>, Option<String>, Option<bool>);
