//! This module contains the glue code between our database and the index files
//! and is used by the corresponding background jobs to generate the
//! index files.

use crate::models::{Crate, Dependency, Version};
use crate::schema::crates;
use anyhow::Context;
use crates_io_index::features::split_features;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use sentry::Level;

#[instrument(skip_all, fields(krate.name = ?name))]
pub async fn get_index_data(
    name: &str,
    conn: &mut AsyncPgConnection,
) -> anyhow::Result<Option<String>> {
    debug!("Looking up crate by name");
    let krate = crates::table
        .select(Crate::as_select())
        .filter(crates::name.eq(name))
        .first::<Crate>(conn)
        .await
        .optional();

    let Some(krate) = krate? else {
        return Ok(None);
    };

    debug!("Gathering remaining index data");
    let crates = index_metadata(&krate, conn)
        .await
        .context("Failed to gather index metadata")?;

    // This can sometimes happen when we delete versions upon owner request
    // but don't realize that the crate is now left with no versions at all.
    //
    // In this case we will delete the crate from the index and log a warning to
    // Sentry to clean this up in the database.
    if crates.is_empty() {
        let message = format!("Crate `{name}` has no versions left");
        sentry::capture_message(&message, Level::Warning);

        return Ok(None);
    }

    debug!("Serializing index data");
    let mut bytes = Vec::new();
    crates_io_index::write_crates(&crates, &mut bytes)
        .context("Failed to serialize index metadata")?;

    let str = String::from_utf8(bytes).context("Failed to decode index metadata as utf8")?;

    Ok(Some(str))
}

/// Gather all the necessary data to write an index metadata file
pub async fn index_metadata(
    krate: &Crate,
    conn: &mut AsyncPgConnection,
) -> QueryResult<Vec<crates_io_index::Crate>> {
    let mut versions: Vec<Version> = Version::belonging_to(krate)
        .select(Version::as_select())
        .load(conn)
        .await?;

    // We sort by `created_at` by default, but since tests run within a
    // single database transaction the versions will all have the same
    // `created_at` timestamp, so we sort by semver as a secondary key.
    versions.sort_by_cached_key(|k| (k.created_at, semver::Version::parse(&k.num).ok()));

    let deps: Vec<(Dependency, String)> = Dependency::belonging_to(&versions)
        .inner_join(crates::table)
        .select((Dependency::as_select(), crates::name))
        .load(conn)
        .await?;

    let deps = deps.grouped_by(&versions);

    versions
        .into_iter()
        .zip(deps)
        .map(|(version, deps)| {
            let mut deps = deps
                .into_iter()
                .map(|(dep, name)| {
                    // If this dependency has an explicit name in `Cargo.toml` that
                    // means that the `name` we have listed is actually the package name
                    // that we're depending on. The `name` listed in the index is the
                    // Cargo.toml-written-name which is what cargo uses for
                    // `--extern foo=...`
                    let (name, package) = match dep.explicit_name {
                        Some(explicit_name) => (explicit_name, Some(name)),
                        None => (name, None),
                    };

                    crates_io_index::Dependency {
                        name,
                        req: dep.req,
                        features: dep.features,
                        optional: dep.optional,
                        default_features: dep.default_features,
                        kind: Some(dep.kind.into()),
                        package,
                        target: dep.target,
                    }
                })
                .collect::<Vec<_>>();

            deps.sort();

            let features = version.features().unwrap_or_default();
            let (features, features2) = split_features(features);

            let (features2, v) = if features2.is_empty() {
                (None, None)
            } else {
                (Some(features2), Some(2))
            };

            let krate = crates_io_index::Crate {
                name: krate.name.clone(),
                vers: version.num.to_string(),
                cksum: version.checksum,
                yanked: Some(version.yanked),
                deps,
                features,
                links: version.links,
                rust_version: version.rust_version,
                features2,
                v,
            };

            Ok(krate)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::schema::users;
    use crate::tests::builders::{CrateBuilder, VersionBuilder};
    use chrono::{Days, Utc};
    use crates_io_test_db::TestDatabase;
    use insta::assert_json_snapshot;

    #[tokio::test]
    async fn test_index_metadata() {
        let test_db = TestDatabase::new();
        let mut conn = test_db.async_connect().await;

        let user_id = diesel::insert_into(users::table)
            .values((
                users::name.eq("user1"),
                users::gh_login.eq("user1"),
                users::gh_id.eq(42),
                users::gh_access_token.eq("some random token"),
            ))
            .returning(users::id)
            .get_result::<i32>(&mut conn)
            .await
            .unwrap();

        let created_at_1 = Utc::now()
            .checked_sub_days(Days::new(14))
            .unwrap()
            .naive_utc();

        let created_at_2 = Utc::now()
            .checked_sub_days(Days::new(7))
            .unwrap()
            .naive_utc();

        let fooo = CrateBuilder::new("foo", user_id)
            .version(VersionBuilder::new("0.1.0"))
            .expect_build(&mut conn)
            .await;

        let metadata = index_metadata(&fooo, &mut conn).await.unwrap();
        assert_json_snapshot!(metadata);

        let bar = CrateBuilder::new("bar", user_id)
            .version(
                VersionBuilder::new("1.0.0-beta.1")
                    .created_at(created_at_1)
                    .yanked(true),
            )
            .version(VersionBuilder::new("1.0.0").created_at(created_at_1))
            .version(
                VersionBuilder::new("2.0.0")
                    .created_at(created_at_2)
                    .dependency(&fooo, None),
            )
            .version(VersionBuilder::new("1.0.1").checksum("0123456789abcdef"))
            .expect_build(&mut conn)
            .await;

        let metadata = index_metadata(&bar, &mut conn).await.unwrap();
        assert_json_snapshot!(metadata);
    }
}
