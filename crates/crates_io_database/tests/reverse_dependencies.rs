//! Tests for the database triggers that maintain the `reverse_dependencies`
//! summary table. Each test drives the source tables with raw inserts/updates
//! and asserts the resulting reverse dependencies, so the trigger behaviour is
//! exercised directly rather than through the HTTP layer.

use crates_io_database::models::{DependencyKind, update_default_version};
use crates_io_database::schema::{
    crate_downloads, crates, dependencies, reverse_dependencies, versions,
};
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use diesel_async::{AsyncConnection, AsyncPgConnection, RunQueryDsl};
use insta::assert_compact_debug_snapshot;

/// A crate's first version becomes its default before any dependency rows
/// exist, so inserting the version creates no reverse dependencies. Inserting
/// the dependency rows is what creates them. This mirrors the publish path,
/// which writes the version and its dependencies in separate statements.
#[tokio::test]
async fn rev_deps_built_on_first_publish() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;

    let c2 = create_crate(&mut conn, "c2").await;
    let version = create_version(&mut conn, c2, "1.0.0").await;

    // No rev deps, because the version has no dependencies yet.
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @"[]");

    // Inserting the dependency row adds the rev dep.
    add_dependency(&mut conn, version, c1, DependencyKind::Normal).await;
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c2", 0)]"#);
}

/// A version that does not become the crate's default contributes no reverse
/// dependencies. Publishing an older `0.9.0` after `1.0.0` leaves `1.0.0` the
/// default, so the dependencies of `0.9.0` produce no edges even after
/// `update_default_version` runs.
#[tokio::test]
async fn rev_dep_not_created_for_non_default_version() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = create_crate(&mut conn, "c2").await;
    create_version(&mut conn, c2, "1.0.0").await;

    // c3's default version (1.0.0) depends on c1.
    let c3 = create_crate(&mut conn, "c3").await;
    let v1 = create_version(&mut conn, c3, "1.0.0").await;
    add_dependency(&mut conn, v1, c1, DependencyKind::Normal).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @"[]");

    // Publish an older 0.9.0 depending on c2. It stays behind 1.0.0, so the
    // default does not move and c2 gains no edge.
    let v2 = create_version(&mut conn, c3, "0.9.0").await;
    add_dependency(&mut conn, v2, c2, DependencyKind::Normal).await;
    update_default_version(c3, &conn).await.unwrap();

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @"[]");
}

/// The `dependent_downloads` ordering key is populated when a reverse
/// dependency is recorded and kept in sync when the dependent's download count
/// changes. A dependent that appears in more than one target's list has every
/// one of its edges refreshed, not just one.
#[tokio::test]
async fn dependent_downloads_tracked_and_kept_in_sync() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = create_crate(&mut conn, "c2").await;
    create_version(&mut conn, c2, "1.0.0").await;

    // c3's version depends on both c1 and c2, so c3 appears in two
    // targets' reverse-dependency lists.
    let c3 = create_crate(&mut conn, "c3").await;
    set_downloads(&mut conn, c3, 10).await;
    let c3_version = create_version(&mut conn, c3, "1.0.0").await;
    add_dependency(&mut conn, c3_version, c1, DependencyKind::Normal).await;
    add_dependency(&mut conn, c3_version, c2, DependencyKind::Normal).await;

    // c4 and c5 depend on c1 only, each with a distinct download count.
    add_dependent(&mut conn, "c4", 30, c1).await;
    add_dependent(&mut conn, "c5", 20, c1).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c4", 30), ("c5", 20), ("c3", 10)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 10)]"#);

    // Bumping a dependent's download count updates the denormalized ordering
    // key in every edge for that dependent, re-sorting each target's list.
    set_downloads(&mut conn, c3, 100).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 100), ("c4", 30), ("c5", 20)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 100)]"#);
}

/// Moving a crate's default version to one with different dependencies rebuilds
/// its reverse-dependency rows: the crate leaves the old targets' lists and
/// joins the new ones.
#[tokio::test]
async fn rev_deps_rebuilt_when_default_version_changes() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = create_crate(&mut conn, "c2").await;
    create_version(&mut conn, c2, "1.0.0").await;

    // c3's default version (1.0.0) depends on c1, so c1 has c3 as a reverse
    // dependency and c2 has none.
    let c3 = create_crate(&mut conn, "c3").await;
    let v1 = create_version(&mut conn, c3, "1.0.0").await;
    add_dependency(&mut conn, v1, c1, DependencyKind::Normal).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @"[]");

    // Publish c3 2.0.0 depending on c2 instead of c1, then move the default
    // version to it, which rebuilds c3's reverse dependencies.
    let v2 = create_version(&mut conn, c3, "2.0.0").await;
    add_dependency(&mut conn, v2, c2, DependencyKind::Normal).await;
    update_default_version(c3, &conn).await.unwrap();

    // The reverse dependencies flipped: c3 left c1's list and joined c2's.
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @"[]");
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 0)]"#);
}

/// Moving a crate's default version to one with no dependencies removes its
/// reverse-dependency rows. The recompute yields no rows for the crate, so the
/// DELETE half of `rebuild_reverse_dependencies` clears the old edges with
/// nothing for the INSERT to add back.
#[tokio::test]
async fn rev_deps_removed_when_default_version_has_no_dependencies() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;

    // c2's default version (1.0.0) depends on c1.
    let c2 = create_crate(&mut conn, "c2").await;
    let version = create_version(&mut conn, c2, "1.0.0").await;
    add_dependency(&mut conn, version, c1, DependencyKind::Normal).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c2", 0)]"#);

    // Publish c2 2.0.0 with no dependencies, then move the default to it.
    create_version(&mut conn, c2, "2.0.0").await;
    update_default_version(c2, &conn).await.unwrap();

    // c2 left c1's list and joined nothing.
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @"[]");
}

/// Yanking a crate's default version removes its reverse dependencies (a yanked
/// default version contributes nothing). Unyanking restores them.
#[tokio::test]
async fn rev_deps_removed_on_yank_and_restored_on_unyank() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;

    let c2 = create_crate(&mut conn, "c2").await;
    let version = create_version(&mut conn, c2, "1.0.0").await;
    add_dependency(&mut conn, version, c1, DependencyKind::Normal).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c2", 0)]"#);

    set_yanked(&mut conn, version, true).await;
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @"[]");

    set_yanked(&mut conn, version, false).await;
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c2", 0)]"#);
}

/// When a default version depends on the same target crate through more than one
/// dependency row (e.g. a normal and a dev dependency), the reverse dependencies
/// collapse to a single row keyed by `(target, dependent)`.
#[tokio::test]
async fn duplicate_dependency_rows_collapse_to_one_rev_dep() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;

    let c2 = create_crate(&mut conn, "c2").await;
    let version = create_version(&mut conn, c2, "1.0.0").await;

    add_dependency(&mut conn, version, c1, DependencyKind::Normal).await;
    add_dependency(&mut conn, version, c1, DependencyKind::Dev).await;

    // The two dependency rows collapse to a single reverse-dependency row.
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c2", 0)]"#);
}

/// Deleting a dependent crate removes its reverse dependencies via the
/// `dependent_crate_id` `ON DELETE CASCADE`.
#[tokio::test]
async fn rev_deps_removed_when_dependent_crate_deleted() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = add_dependent(&mut conn, "c2", 0, c1).await;
    add_dependent(&mut conn, "c3", 0, c1).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0), ("c2", 0)]"#);

    diesel::delete(crates::table.filter(crates::id.eq(c2)))
        .execute(&mut conn)
        .await
        .unwrap();

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
}

/// Deleting a target crate removes its reverse dependencies via the
/// `target_crate_id` `ON DELETE CASCADE`, leaving the dependent's edges to other
/// targets intact.
#[tokio::test]
async fn rev_deps_removed_when_target_crate_deleted() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = create_crate(&mut conn, "c2").await;
    create_version(&mut conn, c2, "1.0.0").await;

    // c3's version depends on both c1 and c2, so it is a reverse dependency of
    // each.
    let c3 = create_crate(&mut conn, "c3").await;
    let version = create_version(&mut conn, c3, "1.0.0").await;
    add_dependency(&mut conn, version, c1, DependencyKind::Normal).await;
    add_dependency(&mut conn, version, c2, DependencyKind::Normal).await;

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 0)]"#);

    diesel::delete(crates::table.filter(crates::id.eq(c1)))
        .execute(&mut conn)
        .await
        .unwrap();

    // Only the edge targeting c1 is gone; c3 remains a reverse dependency of c2.
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 0)]"#);
}

#[tokio::test]
async fn rev_deps_rebuilt_when_default_version_deleted() {
    let test_db = TestDatabase::new();
    let mut conn = test_db.async_connect().await;

    let c1 = create_crate(&mut conn, "c1").await;
    create_version(&mut conn, c1, "1.0.0").await;
    let c2 = create_crate(&mut conn, "c2").await;
    create_version(&mut conn, c2, "1.0.0").await;

    let c3 = create_crate(&mut conn, "c3").await;
    let v1 = create_version(&mut conn, c3, "1.0.0").await;
    add_dependency(&mut conn, v1, c1, DependencyKind::Normal).await;
    let v2 = create_version(&mut conn, c3, "2.0.0").await;
    add_dependency(&mut conn, v2, c2, DependencyKind::Normal).await;
    update_default_version(c3, &conn).await.unwrap();

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @"[]");
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @r#"[("c3", 0)]"#);

    // Mirror the admin `delete_version` flow: delete the default version and
    // reconcile the default in one transaction (the `default_versions` foreign
    // key check is deferred to commit).
    conn.transaction(async |conn| {
        diesel::delete(versions::table.filter(versions::id.eq(v2)))
            .execute(conn)
            .await?;

        update_default_version(c3, conn).await
    })
    .await
    .unwrap();

    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c1).await, @r#"[("c3", 0)]"#);
    assert_compact_debug_snapshot!(stored_dependents(&mut conn, c2).await, @"[]");
}

/// Creates a crate and returns its id.
async fn create_crate(conn: &mut AsyncPgConnection, name: &str) -> i32 {
    diesel::insert_into(crates::table)
        .values(crates::name.eq(name))
        .returning(crates::id)
        .get_result(conn)
        .await
        .unwrap()
}

/// Creates a version of `crate_id` and returns its id.
async fn create_version(conn: &mut AsyncPgConnection, crate_id: i32, num: &str) -> i32 {
    diesel::insert_into(versions::table)
        .values((
            versions::crate_id.eq(crate_id),
            versions::num.eq(num),
            versions::num_no_build.eq(num),
            versions::checksum.eq("0".repeat(64)),
            versions::tar_sha256.eq(vec![0u8; 32]),
            versions::crate_size.eq(0),
        ))
        .returning(versions::id)
        .get_result(conn)
        .await
        .unwrap()
}

/// Adds a dependency from `version_id` onto `target_crate_id` and returns the
/// new `dependencies.id`.
async fn add_dependency(
    conn: &mut AsyncPgConnection,
    version_id: i32,
    target_crate_id: i32,
    kind: DependencyKind,
) -> i32 {
    diesel::insert_into(dependencies::table)
        .values((
            dependencies::version_id.eq(version_id),
            dependencies::crate_id.eq(target_crate_id),
            dependencies::req.eq(">= 0"),
            dependencies::optional.eq(false),
            dependencies::default_features.eq(false),
            dependencies::features.eq(Vec::<String>::new()),
            dependencies::kind.eq(kind),
        ))
        .returning(dependencies::id)
        .get_result(conn)
        .await
        .unwrap()
}

/// Creates a crate `name` with `downloads` total downloads whose single `1.0.0`
/// version depends on `target_crate_id`, and returns the new crate's id.
async fn add_dependent(
    conn: &mut AsyncPgConnection,
    name: &str,
    downloads: i64,
    target_crate_id: i32,
) -> i32 {
    let krate = create_crate(conn, name).await;
    set_downloads(conn, krate, downloads).await;
    let version = create_version(conn, krate, "1.0.0").await;
    add_dependency(conn, version, target_crate_id, DependencyKind::Normal).await;
    krate
}

/// Sets a crate's total download count.
async fn set_downloads(conn: &mut AsyncPgConnection, crate_id: i32, downloads: i64) {
    diesel::update(crate_downloads::table.filter(crate_downloads::crate_id.eq(crate_id)))
        .set(crate_downloads::downloads.eq(downloads))
        .returning(crate_downloads::crate_id)
        .get_result::<i32>(conn)
        .await
        .unwrap();
}

/// Sets a version's yank status.
async fn set_yanked(conn: &mut AsyncPgConnection, version_id: i32, yanked: bool) {
    diesel::update(versions::table.filter(versions::id.eq(version_id)))
        .set(versions::yanked.eq(yanked))
        .execute(conn)
        .await
        .unwrap();
}

/// Loads the stored reverse dependencies of `target_crate_id`, returning each
/// dependent crate's name and denormalized download count in the order the
/// serve index yields them (`dependent_downloads` descending, then
/// `dependent_crate_id` descending).
async fn stored_dependents(
    conn: &mut AsyncPgConnection,
    target_crate_id: i32,
) -> Vec<(String, i64)> {
    reverse_dependencies::table
        .inner_join(crates::table.on(crates::id.eq(reverse_dependencies::dependent_crate_id)))
        .filter(reverse_dependencies::target_crate_id.eq(target_crate_id))
        .order((
            reverse_dependencies::dependent_downloads.desc(),
            reverse_dependencies::dependent_crate_id.desc(),
        ))
        .select((crates::name, reverse_dependencies::dependent_downloads))
        .load(conn)
        .await
        .unwrap()
}
