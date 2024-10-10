use crate::schema::versions;
use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::EncodableVersion;
use diesel::{prelude::*, update};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn versions() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_versions", user.id)
            .version("0.5.1")
            .version(VersionBuilder::new("1.0.0").rust_version("1.64"))
            .version("0.5.0")
            .expect_build(conn);
        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();
    });

    let response = anon.get::<()>("/api/v1/crates/foo_versions/versions").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".versions[].created_at" => "[datetime]",
        ".versions[].updated_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (_, anon) = TestApp::init().empty();

    let response = anon.get::<()>("/api/v1/crates/unknown/versions").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r###"{"errors":[{"detail":"crate `unknown` does not exist"}]}"###);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_sorting() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    let versions = [
        "1.0.0-alpha",
        "2.0.0-alpha",
        "1.0.0-beta",
        "1.0.0-alpha.1",
        "1.0.0-beta.2",
        "1.0.0-alpha.beta",
        "1.0.0-beta.11",
        "1.0.0-rc.1",
        "1.0.0",
    ];
    app.db(|conn| {
        let mut builder = CrateBuilder::new("foo_versions", user.id);
        for version in versions {
            builder = builder.version(version);
        }
        builder.expect_build(conn);
        // Make version 1.0.0-beta.2 and 1.0.0-alpha.beta mimic versions created at same time,
        // but 1.0.0-alpha.beta owns larger id number
        let versions_aliased = diesel::alias!(versions as versions_aliased);
        let created_at_by_num = |num: &str| {
            versions_aliased
                .filter(versions_aliased.field(versions::num).eq(num.to_owned()))
                .select(versions_aliased.field(versions::created_at))
                .single_value()
        };
        update(versions::table)
            .filter(versions::num.eq("1.0.0-beta.2"))
            .set(versions::created_at.eq(created_at_by_num("1.0.0-alpha.beta").assume_not_null()))
            .execute(conn)
            .unwrap();

        // An additional crate to guarantee the accuracy of the response dataset and its total
        CrateBuilder::new("bar_versions", user.id)
            .version("0.0.1")
            .expect_build(conn);
    });

    // Sort by semver
    let url = "/api/v1/crates/foo_versions/versions?sort=semver";
    let json: AllVersions = anon.get(url).await.good();
    let expects = [
        "2.0.0-alpha",
        "1.0.0",
        "1.0.0-rc.1",
        "1.0.0-beta.11",
        "1.0.0-beta.2",
        "1.0.0-beta",
        "1.0.0-alpha.beta",
        "1.0.0-alpha.1",
        "1.0.0-alpha",
    ];
    for (num, expect) in nums(&json.versions).iter().zip(expects) {
        assert_eq!(num, expect);
    }
    let (resp, calls) = page_with_seek(&anon, url).await;
    for (json, expect) in resp.iter().zip(expects) {
        assert_eq!(json.versions[0].num, expect);
        assert_eq!(json.meta.total as usize, expects.len());
        assert_eq!(
            json.meta.release_tracks,
            Some(json!({"1": {"highest": "1.0.0"}}))
        );
    }
    assert_eq!(calls as usize, expects.len() + 1);

    // Sort by date
    let url = "/api/v1/crates/foo_versions/versions?sort=date";
    let json: AllVersions = anon.get(url).await.good();
    let expects = versions.iter().cloned().rev().collect::<Vec<_>>();
    for (num, expect) in nums(&json.versions).iter().zip(&expects) {
        assert_eq!(num, *expect);
    }
    let (resp, calls) = page_with_seek(&anon, url).await;
    for (json, expect) in resp.iter().zip(&expects) {
        assert_eq!(json.versions[0].num, *expect);
        assert_eq!(json.meta.total as usize, expects.len());
    }
    assert_eq!(calls as usize, expects.len() + 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_seek_based_pagination_semver_sorting() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_versions", user.id)
            .version(VersionBuilder::new("0.5.1").yanked(true))
            .version(VersionBuilder::new("1.0.0").rust_version("1.64"))
            .version("0.5.0")
            .expect_build(conn);
        // Make version 1.0.0 mimic a version published before we started recording who published
        // versions
        let none: Option<i32> = None;
        update(versions::table)
            .filter(versions::num.eq("1.0.0"))
            .set(versions::published_by.eq(none))
            .execute(conn)
            .unwrap();
    });

    let url = "/api/v1/crates/foo_versions/versions";
    let expects = ["1.0.0", "0.5.1", "0.5.0"];
    let release_tracks = Some(json!({
        "1": {"highest": "1.0.0"},
        "0.5": {"highest": "0.5.0"}
    }));

    // per_page larger than the number of versions
    let json: VersionList = anon
        .get_with_query(url, "per_page=10&sort=semver")
        .await
        .good();
    assert_eq!(nums(&json.versions), expects);
    assert_eq!(json.meta.total as usize, expects.len());
    assert_eq!(json.meta.release_tracks, release_tracks);

    let json: VersionList = anon
        .get_with_query(url, "per_page=1&sort=semver")
        .await
        .good();
    assert_eq!(nums(&json.versions), expects[0..1]);
    assert_eq!(json.meta.total as usize, expects.len());
    assert_eq!(json.meta.release_tracks, release_tracks);

    let seek = json
        .meta
        .next_page
        .map(|s| s.split_once("seek=").unwrap().1.to_owned())
        .map(|p| p.split_once('&').map(|t| t.0.to_owned()).unwrap_or(p))
        .unwrap();

    // per_page larger than the number of remain versions
    let json: VersionList = anon
        .get_with_query(url, &format!("per_page=5&sort=semver&seek={seek}"))
        .await
        .good();
    assert_eq!(nums(&json.versions), expects[1..]);
    assert!(json.meta.next_page.is_none());
    assert_eq!(json.meta.total as usize, expects.len());
    assert_eq!(json.meta.release_tracks, release_tracks);

    // per_page euqal to the number of remain versions
    let json: VersionList = anon
        .get_with_query(url, &format!("per_page=2&sort=semver&seek={seek}"))
        .await
        .good();
    assert_eq!(nums(&json.versions), expects[1..]);
    assert!(json.meta.next_page.is_some());
    assert_eq!(json.meta.total as usize, expects.len());
    assert_eq!(json.meta.release_tracks, release_tracks);

    // A decodable seek value, MTAwCg (100), but doesn't actually exist
    let json: VersionList = anon
        .get_with_query(url, "per_page=10&sort=semver&seek=MTAwCg")
        .await
        .good();
    assert_eq!(json.versions.len(), 0);
    assert!(json.meta.next_page.is_none());
    assert_eq!(json.meta.total, 0);
    assert_eq!(json.meta.release_tracks, release_tracks);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_seek_parameter() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();
    app.db(|conn| {
        CrateBuilder::new("foo_versions", user.id).expect_build(conn);
    });

    let url = "/api/v1/crates/foo_versions/versions";
    // Sort by semver
    let response = anon
        .get_with_query::<()>(url, "per_page=1&sort=semver&seek=broken")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid seek parameter"}]}"#);

    // Sort by date
    let response = anon
        .get_with_query::<()>(url, "per_page=1&sort=date&seek=broken")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid seek parameter"}]}"#);

    // broken seek but without per_page parameter should be ok
    // since it's not consider as seek-based pagination
    let response = anon.get_with_query::<()>(url, "seek=broken").await;
    assert_eq!(response.status(), StatusCode::OK);
}

#[derive(Debug, Deserialize)]
pub struct AllVersions {
    pub versions: Vec<EncodableVersion>,
}

#[derive(Debug, Deserialize)]
pub struct VersionList {
    pub versions: Vec<EncodableVersion>,
    pub meta: ResponseMeta,
}

#[derive(Debug, Deserialize)]
pub struct ResponseMeta {
    pub total: i64,
    pub next_page: Option<String>,
    pub release_tracks: Option<serde_json::Value>,
}

fn nums(versions: &[EncodableVersion]) -> Vec<String> {
    versions.iter().map(|v| v.num.to_owned()).collect()
}

async fn page_with_seek<U: RequestHelper>(anon: &U, url: &str) -> (Vec<VersionList>, i32) {
    let (url_without_query, query) = url.split_once('?').unwrap_or((url, ""));
    let mut url = Some(format!("{url_without_query}?per_page=1&{query}"));
    let mut results = Vec::new();
    let mut calls = 0;
    while let Some(current_url) = url.take() {
        let resp: VersionList = anon.get(&current_url).await.good();
        calls += 1;
        if calls > 200 {
            panic!("potential infinite loop detected!");
        }

        if let Some(ref new_url) = resp.meta.next_page {
            assert!(new_url.contains("seek="));
            assert_that!(resp.versions, len(eq(1)));
            url = Some(format!("{url_without_query}{}", new_url));
            assert_ne!(resp.meta.total, 0)
        } else {
            assert_that!(resp.versions, empty());
            assert_eq!(resp.meta.total, 0)
        }
        results.push(resp);
    }
    (results, calls)
}
