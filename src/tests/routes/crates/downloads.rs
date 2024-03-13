use crate::builders::{CrateBuilder, VersionBuilder};
use crate::util::{MockAnonymousUser, RequestHelper, TestApp};
use chrono::{Duration, Utc};
use crates_io::schema::{crates, version_downloads, versions};
use crates_io::views::EncodableVersionDownload;
use diesel::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[derive(Deserialize)]
struct Downloads {
    version_downloads: Vec<EncodableVersionDownload>,
}

fn save_version_downloads(
    crate_name: &str,
    version: &str,
    num_downloads: i32,
    conn: &mut PgConnection,
) {
    let version_id = versions::table
        .select(versions::id)
        .left_join(crates::table)
        .filter(crates::name.eq(crate_name))
        .filter(versions::num.eq(version))
        .first::<i32>(conn)
        .unwrap();

    diesel::insert_into(version_downloads::table)
        .values((
            version_downloads::version_id.eq(version_id),
            version_downloads::downloads.eq(num_downloads),
        ))
        .execute(conn)
        .unwrap();
}

#[track_caller]
pub fn assert_dl_count(
    anon: &MockAnonymousUser,
    name_and_version: &str,
    query: Option<&str>,
    count: i32,
) {
    let url = format!("/api/v1/crates/{name_and_version}/downloads");
    let downloads: Downloads = if let Some(query) = query {
        anon.get_with_query(&url, query).good()
    } else {
        anon.get(&url).good()
    };
    let total_downloads = downloads
        .version_downloads
        .iter()
        .map(|vd| vd.downloads)
        .sum::<i32>();
    assert_eq!(total_downloads, count);
}

pub fn download(client: &impl RequestHelper, name_and_version: &str) {
    let url = format!("/api/v1/crates/{name_and_version}/download");
    let response = client.get::<()>(&url);
    assert_eq!(response.status(), StatusCode::FOUND);
}

#[test]
fn test_download() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    app.db(|conn| {
        CrateBuilder::new("foo_download", user.id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    // TODO: test the with_json code path
    download(&anon, "foo_download/1.0.0");

    // No downloads are counted until the corresponding log files are processed.
    assert_dl_count(&anon, "foo_download/1.0.0", None, 0);
    assert_dl_count(&anon, "foo_download", None, 0);

    app.db(|conn| save_version_downloads("foo_download", "1.0.0", 1, conn));

    // Now that the counters are persisted the download counts show up.
    assert_dl_count(&anon, "foo_download/1.0.0", None, 1);
    assert_dl_count(&anon, "foo_download", None, 1);

    let yesterday = (Utc::now().date_naive() + Duration::days(-1)).format("%F");
    let query = format!("before_date={yesterday}");
    assert_dl_count(&anon, "foo_download/1.0.0", Some(&query), 0);
    // crate/downloads always returns the last 90 days and ignores date params
    assert_dl_count(&anon, "foo_download", Some(&query), 1);

    let tomorrow = (Utc::now().date_naive() + Duration::days(1)).format("%F");
    let query = format!("before_date={tomorrow}");
    assert_dl_count(&anon, "foo_download/1.0.0", Some(&query), 1);
    assert_dl_count(&anon, "foo_download", Some(&query), 1);
}

#[test]
fn test_download_with_counting_via_cdn() {
    let (app, anon, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("foo", user.as_model().id)
            .version(VersionBuilder::new("1.0.0"))
            .expect_build(conn);
    });

    download(&anon, "foo/1.0.0");

    assert_dl_count(&anon, "foo/1.0.0", None, 0);
    assert_dl_count(&anon, "foo", None, 0);
}

#[test]
fn test_crate_downloads() {
    let (app, anon, cookie) = TestApp::init().with_user();

    app.db(|conn| {
        let user_id = cookie.as_model().id;
        CrateBuilder::new("foo", user_id)
            .version("1.0.0")
            .version("1.1.0")
            .expect_build(conn);
    });

    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.1.0");

    app.db(|conn| {
        save_version_downloads("foo", "1.0.0", 3, conn);
        save_version_downloads("foo", "1.1.0", 1, conn);
    });

    let response = anon.get::<()>("/api/v1/crates/foo/downloads");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    assert_json_snapshot!(json, {
        ".version_downloads[].date" => "[date]",
    });

    // check different crate name
    let response = anon.get::<()>("/api/v1/crates/bar/downloads");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(
        response.text(),
        @r###"{"errors":[{"detail":"crate `bar` does not exist"}]}"###
    );

    // check non-canonical crate name
    let response = anon.get::<()>("/api/v1/crates/FOO/downloads");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json(), json);
}

#[test]
fn test_version_downloads() {
    let (app, anon, cookie) = TestApp::init().with_user();

    app.db(|conn| {
        let user_id = cookie.as_model().id;
        CrateBuilder::new("foo", user_id)
            .version("1.0.0")
            .version("1.1.0")
            .expect_build(conn);
    });

    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.0.0");
    download(&anon, "foo/1.1.0");

    app.db(|conn| {
        save_version_downloads("foo", "1.0.0", 3, conn);
        save_version_downloads("foo", "1.1.0", 1, conn);
    });

    let response = anon.get::<()>("/api/v1/crates/foo/1.0.0/downloads");
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.json();
    assert_json_snapshot!(json, {
        ".version_downloads[].date" => "[date]",
    });

    // check different crate name
    let response = anon.get::<()>("/api/v1/crates/bar/1.0.0/downloads");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(
        response.text(),
        @r###"{"errors":[{"detail":"crate `bar` does not exist"}]}"###
    );

    // check non-canonical crate name
    let response = anon.get::<()>("/api/v1/crates/FOO/1.0.0/downloads");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.json(), json);

    // check missing version
    let response = anon.get::<()>("/api/v1/crates/foo/2.0.0/downloads");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(
        response.text(),
        @r###"{"errors":[{"detail":"crate `foo` does not have a version `2.0.0`"}]}"###
    );

    // check invalid version
    let response = anon.get::<()>("/api/v1/crates/foo/invalid-version/downloads");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(
        response.text(),
        @r###"{"errors":[{"detail":"crate `foo` does not have a version `invalid-version`"}]}"###
    );
}
