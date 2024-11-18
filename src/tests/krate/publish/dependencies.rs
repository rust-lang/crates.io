use crate::tests::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn invalid_dependency_name() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(PublishBuilder::new("foo", "1.0.0").dependency(DependencyBuilder::new("🦀")))
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `🦀` in dependency name: `🦀`, the first character must be an ASCII character"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_with_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("package-name").rename("my-name");

    let crate_to_publish = PublishBuilder::new("new-krate", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).await.good();

    let crates = app.crates_from_index_head("new-krate");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_dependency_rename() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let response = token
        .publish_crate(
            PublishBuilder::new("new-krate", "1.0.0")
                .dependency(DependencyBuilder::new("package-name").rename("💩")),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `💩` in dependency name: `💩`, the first character must be an ASCII character, or `_`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_dependency_name_starts_with_digit() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let response = token
        .publish_crate(
            PublishBuilder::new("new-krate", "1.0.0")
                .dependency(DependencyBuilder::new("package-name").rename("1-foo")),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"the name `1-foo` cannot be used as a dependency name, the name cannot start with a digit"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_dependency_name_contains_unicode_chars() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let response = token
        .publish_crate(
            PublishBuilder::new("new-krate", "1.0.0")
                .dependency(DependencyBuilder::new("package-name").rename("foo-🦀-bar")),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `🦀` in dependency name: `foo-🦀-bar`, characters must be an ASCII alphanumeric characters, `-`, or `_`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_too_long_dependency_name() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let response = token
        .publish_crate(
            PublishBuilder::new("new-krate", "1.0.0")
                .dependency(DependencyBuilder::new("package-name").rename("f".repeat(65).as_str())),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"the dependency name `fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff` is too long (max 64 characters)"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_dependency_name() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let response = token
        .publish_crate(
            PublishBuilder::new("new-krate", "1.0.0")
                .dependency(DependencyBuilder::new("package-name").rename("")),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"dependency name cannot be empty"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_with_underscore_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new-krate can depend on it
    CrateBuilder::new("package-name", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("package-name").rename("_my-name");

    let crate_to_publish = PublishBuilder::new("new-krate", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).await.good();

    let crates = app.crates_from_index_head("new-krate");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_dependency() {
    use crate::tests::routes::crates::versions::dependencies::Deps;

    let (app, anon, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new_dep can depend on it
    // The name choice of `foo-dep` is important! It has the property of
    // name != canon_crate_name(name) and is a regression test for
    // https://github.com/rust-lang/crates.io/issues/651
    CrateBuilder::new("foo-dep", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("foo-dep").version_req("1.0.0");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);

    token.publish_crate(crate_to_publish).await.good();

    let dependencies = anon
        .get::<Deps>("/api/v1/crates/new_dep/1.0.0/dependencies")
        .await
        .good()
        .dependencies;

    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].crate_id, "foo-dep");
    assert_eq!(dependencies[0].req, "^1.0.0");

    let crates = app.crates_from_index_head("new_dep");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_broken_dependency_requirement() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new_dep can depend on it
    // The name choice of `foo-dep` is important! It has the property of
    // name != canon_crate_name(name) and is a regression test for
    // https://github.com/rust-lang/crates.io/issues/651
    CrateBuilder::new("foo-dep", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("foo-dep").version_req("broken");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"\"broken\" is an invalid version requirement"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn reject_new_krate_with_non_exact_dependency() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("foo-dep", user.as_model().id).expect_build(&mut conn);

    // Use non-exact name for the dependency
    let dependency = DependencyBuilder::new("foo_dep");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"no known crate named `foo_dep`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_crate_allow_empty_alternative_registry_dependency() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("foo-dep", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("foo-dep").registry("");
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).await.good();
}

#[tokio::test(flavor = "multi_thread")]
async fn reject_new_crate_with_alternative_registry_dependency() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let dependency =
        DependencyBuilder::new("dep").registry("https://server.example/path/to/registry");

    let crate_to_publish =
        PublishBuilder::new("depends-on-alt-registry", "1.0.0").dependency(dependency);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Dependency `dep` is hosted on another registry. Cross-registry dependencies are not permitted on crates.io."}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_with_wildcard_dependency() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert a crate directly into the database so that new_wild can depend on it
    CrateBuilder::new("foo_wild", user.as_model().id).expect_build(&mut conn);

    let dependency = DependencyBuilder::new("foo_wild").version_req("*");

    let crate_to_publish = PublishBuilder::new("new_wild", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"wildcard (`*`) dependency constraints are not allowed on crates.io. Crate with this problem: `foo_wild` See https://doc.rust-lang.org/cargo/faq.html#can-libraries-use--as-a-version-for-their-dependencies for more information"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_dependency_missing() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    // Deliberately not inserting this crate in the database to test behavior when a dependency
    // doesn't exist!
    let dependency = DependencyBuilder::new("bar_missing");
    let crate_to_publish = PublishBuilder::new("foo_missing", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"no known crate named `bar_missing`"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn new_krate_sorts_deps() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    // Insert crates directly into the database so that two-deps can depend on it
    CrateBuilder::new("dep-a", user.as_model().id).expect_build(&mut conn);
    CrateBuilder::new("dep-b", user.as_model().id).expect_build(&mut conn);

    let dep_a = DependencyBuilder::new("dep-a");
    let dep_b = DependencyBuilder::new("dep-b");

    // Add the deps in reverse order to ensure they get sorted.
    let crate_to_publish = PublishBuilder::new("two-deps", "1.0.0")
        .dependency(dep_b)
        .dependency(dep_a);
    token.publish_crate(crate_to_publish).await.good();

    let crates = app.crates_from_index_head("two-deps");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_feature_name() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let response = token
        .publish_crate(
            PublishBuilder::new("foo", "1.0.0")
                .dependency(DependencyBuilder::new("bar").add_feature("🍺")),
        )
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `🍺` in feature `🍺`, the first character must be a Unicode XID start character or digit (most letters or `_` or `0` to `9`)"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn test_dep_limit() {
    let (app, _, user, token) = TestApp::full()
        .with_config(|config| config.max_dependencies = 1)
        .with_token()
        .await;

    let mut conn = app.db_conn();

    CrateBuilder::new("dep-a", user.as_model().id).expect_build(&mut conn);
    CrateBuilder::new("dep-b", user.as_model().id).expect_build(&mut conn);

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0")
        .dependency(DependencyBuilder::new("dep-a"))
        .dependency(DependencyBuilder::new("dep-b"));

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crates.io only allows a maximum number of 1 dependencies.\n\nIf you have a use case that requires an increase of this limit, please send us an email to help@crates.io to discuss the details."}]}"#);

    let crate_to_publish =
        PublishBuilder::new("foo", "1.0.0").dependency(DependencyBuilder::new("dep-a"));

    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });
}
