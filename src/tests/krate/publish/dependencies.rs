use crate::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_json_snapshot;

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
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "new-krate");
    assert_eq!(crates[0].vers, "1.0.0");
    assert_eq!(crates[0].deps.len(), 1);
    assert_eq!(crates[0].deps[0].name, "my-name");
    assert_eq!(crates[0].deps[0].package.as_ref().unwrap(), "package-name");
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
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
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
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "new-krate");
    assert_eq!(crates[0].vers, "1.0.0");
    assert_eq!(crates[0].deps.len(), 1);
    assert_eq!(crates[0].deps[0].name, "_my-name");
    assert_eq!(crates[0].deps[0].package.as_ref().unwrap(), "package-name");
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
    assert_eq!(dependencies[0].req, "1.0.0");
}

#[test]
fn new_krate_with_broken_dependency_requirement() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new_dep can depend on it
        // The name choice of `foo-dep` is important! It has the property of
        // name != canon_crate_name(name) and is a regression test for
        // https://github.com/rust-lang/crates.io/issues/651
        CrateBuilder::new("foo-dep", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("foo-dep").version_req("1.2.3");

    let crate_to_publish = PublishBuilder::new("new_dep", "1.0.0").dependency(dependency);

    // create a request body with `version_req: "broken"`
    let (json, tarball) = crate_to_publish.build();
    let new_json = json.replace(r#""version_req":"1.2.3""#, r#""version_req":"broken""#);
    assert_ne!(json, new_json);
    let body = PublishBuilder::create_publish_body(&new_json, &tarball);

    let response = token
        .put::<serde_json::Value>("/api/v1/crates/new", &body)
        .good();

    assert_eq!(
        response,
        json!({"errors": [{"detail": "invalid upload request: invalid value: string \"broken\", expected a valid version req at line 1 column 136"}]})
    );

    assert!(app.stored_files().is_empty());
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
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "no known crate named `foo_dep`" }] })
    );

    assert!(app.stored_files().is_empty());
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
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "Dependency `dep` is hosted on another registry. Cross-registry dependencies are not permitted on crates.io." }] })
    );

    assert!(app.stored_files().is_empty());
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
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "wildcard (`*`) dependency constraints are not allowed \
                        on crates.io. Crate with this problem: `foo_wild` See https://doc.rust-lang.org/cargo/faq.html#can-\
                        libraries-use--as-a-version-for-their-dependencies for more \
                        information" }] })
    );

    assert!(app.stored_files().is_empty());
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
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "no known crate named `bar_missing`" }] })
    );

    assert!(app.stored_files().is_empty());
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
    assert!(crates.len() == 1);
    let deps = &crates[0].deps;
    assert!(deps.len() == 2);
    assert_eq!(deps[0].name, "dep-a");
    assert_eq!(deps[1].name, "dep-b");
}
