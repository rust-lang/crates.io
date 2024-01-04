use crate::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn invalid_dependency_name() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0").dependency(DependencyBuilder::new("🦀")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_with_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("package-name").rename("my-name");

    let crate_to_publish = PublishBuilder::new("new-krate", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("new-krate");
    assert_json_snapshot!(crates);
}

#[test]
fn invalid_dependency_rename() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let response = token.publish_crate(
        PublishBuilder::new("new-krate", "1.0.0")
            .dependency(DependencyBuilder::new("package-name").rename("💩")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_dependency_name_starts_with_digit() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let response = token.publish_crate(
        PublishBuilder::new("new-krate", "1.0.0")
            .dependency(DependencyBuilder::new("package-name").rename("1-foo")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_dependency_name_contains_unicode_chars() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let response = token.publish_crate(
        PublishBuilder::new("new-krate", "1.0.0")
            .dependency(DependencyBuilder::new("package-name").rename("foo-🦀-bar")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn invalid_too_long_dependency_name() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let response =
        token
            .publish_crate(PublishBuilder::new("new-krate", "1.0.0").dependency(
                DependencyBuilder::new("package-name").rename("f".repeat(65).as_str()),
            ));
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn empty_dependency_name() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });
    let response = token.publish_crate(
        PublishBuilder::new("new-krate", "1.0.0")
            .dependency(DependencyBuilder::new("package-name").rename("")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_with_underscore_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("package-name").rename("_my-name");

    let crate_to_publish = PublishBuilder::new("new-krate", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("new-krate");
    assert_json_snapshot!(crates);
}

#[test]
fn new_krate_with_dependency() {
    use crate::routes::crates::versions::dependencies::Deps;

    let (app, anon, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_dep can depend on it
        // The name choice of `foo-dep` is important! It has the property of
        // name != canon_crate_name(name) and is a regression test for
        // https://github.com/rust-lang/crates.io/issues/651
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep").version_req("1.0.0");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);

    token.publish_crate(crate_to_publish).good();

    let dependencies = anon
        .get::<Deps>("/api/v1/crates/new_dep/1.0.0/dependencies")
        .good()
        .dependencies;

    assert_eq!(dependencies.len(), 1);
    assert_eq!(dependencies[0].crate_id, "foo-dep");
    assert_eq!(dependencies[0].req, "^1.0.0");

    let crates = app.crates_from_index_head("new_dep");
    assert_json_snapshot!(crates);
}

#[test]
fn new_krate_with_broken_dependency_requirement() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_dep can depend on it
        // The name choice of `foo-dep` is important! It has the property of
        // name != canon_crate_name(name) and is a regression test for
        // https://github.com/rust-lang/crates.io/issues/651
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep").version_req("broken");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn reject_new_krate_with_non_exact_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    // Use non-exact name for the dependency
    let dependency = DependencyBuilder::new("foo_dep");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_crate_allow_empty_alternative_registry_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep").registry("");
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").dependency(dependency);
    token.publish_crate(crate_to_publish).good();
}

#[test]
fn reject_new_crate_with_alternative_registry_dependency() {
    let (app, _, _, token) = TestApp::full().with_token();

    let dependency =
        DependencyBuilder::new("dep").registry("https://server.example/path/to/registry");

    let crate_to_publish =
        PublishBuilder::new("depends-on-alt-registry", "1.0.0").dependency(dependency);
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_krate_with_wildcard_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_wild can depend on it
        CrateBuilder::new("foo_wild", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo_wild").version_req("*");

    let crate_to_publish = PublishBuilder::new("new_wild", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_krate_dependency_missing() {
    let (app, _, _, token) = TestApp::full().with_token();

    // Deliberately not inserting this crate in the database to test behavior when a dependency
    // doesn't exist!
    let dependency = DependencyBuilder::new("bar_missing");
    let crate_to_publish = PublishBuilder::new("foo_missing", "1.0.0").dependency(dependency);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}

#[test]
fn new_krate_sorts_deps() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert crates directly into the database so that two-deps can depend on it
        CrateBuilder::new("dep-a", user.as_model().id).expect_build(conn);
        CrateBuilder::new("dep-b", user.as_model().id).expect_build(conn);
    });

    let dep_a = DependencyBuilder::new("dep-a");
    let dep_b = DependencyBuilder::new("dep-b");

    // Add the deps in reverse order to ensure they get sorted.
    let crate_to_publish = PublishBuilder::new("two-deps", "1.0.0")
        .dependency(dep_b)
        .dependency(dep_a);
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("two-deps");
    assert_json_snapshot!(crates);
}

#[test]
fn invalid_feature_name() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.publish_crate(
        PublishBuilder::new("foo", "1.0.0")
            .dependency(DependencyBuilder::new("bar").add_feature("🍺")),
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json());
    assert_that!(app.stored_files(), empty());
}
