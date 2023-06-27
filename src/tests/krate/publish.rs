use crate::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::new_category;
use crate::util::insta::assert_yaml_snapshot;
use crate::util::{RequestHelper, TestApp};
use crates_io::controllers::krate::publish::{
    missing_metadata_error_message, MISSING_RIGHTS_ERROR_MESSAGE, WILDCARD_ERROR_MESSAGE,
};
use crates_io::models::krate::MAX_NAME_LENGTH;
use crates_io::schema::{api_tokens, emails, versions_published_by};
use crates_io::views::GoodCrate;
use diesel::{delete, update, ExpressionMethods, QueryDsl, RunQueryDsl};
use flate2::write::GzEncoder;
use flate2::Compression;
use http::StatusCode;
use std::collections::BTreeMap;
use std::io::Read;
use std::iter::FromIterator;
use std::time::Duration;
use std::{io, thread};

#[test]
fn uploading_new_version_touches_crate() {
    use crate::builders::PublishBuilder;
    use crate::util::{RequestHelper, TestApp};
    use crate::CrateResponse;
    use crates_io::schema::crates;
    use diesel::dsl::*;
    use diesel::{ExpressionMethods, RunQueryDsl};

    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at").version("1.0.0");
    user.publish_crate(crate_to_publish).good();

    app.db(|conn| {
        diesel::update(crates::table)
            .set(crates::updated_at.eq(crates::updated_at - 1.hour()))
            .execute(conn)
            .unwrap();
    });

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_before = json.krate.updated_at;

    let crate_to_publish = PublishBuilder::new("foo_versions_updated_at").version("2.0.0");
    user.publish_crate(crate_to_publish).good();

    let json: CrateResponse = user.show_crate("foo_versions_updated_at");
    let updated_at_after = json.krate.updated_at;

    assert_ne!(updated_at_before, updated_at_after);
}

#[test]
fn new_wrong_token() {
    let (app, anon, _, token) = TestApp::full().with_token();

    // Try to publish without a token
    let crate_to_publish = PublishBuilder::new("foo");
    let response = anon.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "must be logged in to perform that action" }] })
    );

    // Try to publish with the wrong token (by changing the token in the database)
    app.db(|conn| {
        diesel::update(api_tokens::table)
            .set(api_tokens::token.eq(b"bad" as &[u8]))
            .execute(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "must be logged in to perform that action" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn invalid_names() {
    let (app, _, _, token) = TestApp::full().with_token();

    let bad_name = |name: &str, error_message: &str| {
        let crate_to_publish = PublishBuilder::new(name).version("1.0.0");
        let response = token.publish_crate(crate_to_publish);
        assert_eq!(response.status(), StatusCode::OK);

        let json = response.into_json();
        let json = json.as_object().unwrap();
        let errors = json.get("errors").unwrap().as_array().unwrap();
        let first_error = errors.first().unwrap().as_object().unwrap();
        let detail = first_error.get("detail").unwrap().as_str().unwrap();
        assert!(detail.contains(error_message), "{detail:?}");
    };

    let error_message = "expected a valid crate name";
    bad_name("", error_message);
    bad_name("foo bar", error_message);
    bad_name(&"a".repeat(MAX_NAME_LENGTH + 1), error_message);
    bad_name("snow☃", error_message);
    bad_name("áccênts", error_message);

    let error_message = "cannot upload a crate with a reserved name";
    bad_name("std", error_message);
    bad_name("STD", error_message);
    bad_name("compiler-rt", error_message);
    bad_name("compiler_rt", error_message);
    bad_name("coMpiLer_Rt", error_message);

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate() {
    let (app, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_new").version("1.0.0");
    let json: GoodCrate = user.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");

    let crates = app.crates_from_index_head("foo_new");
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "foo_new");
    assert_eq!(crates[0].vers, "1.0.0");
    assert!(crates[0].deps.is_empty());
    assert_eq!(
        crates[0].cksum,
        "acb5604b126ac894c1eb11c4575bf2072fea61232a888e453770c79d7ed56419"
    );

    let files = app.stored_files();
    assert_eq!(files, vec!["crates/foo_new/foo_new-1.0.0.crate"]);

    app.db(|conn| {
        let email: String = versions_published_by::table
            .select(versions_published_by::email)
            .first(conn)
            .unwrap();
        assert_eq!(email, "something@example.com");
    });
}

#[test]
fn new_krate_with_token() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_new").version("1.0.0");
    let json: GoodCrate = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_new");
    assert_eq!(json.krate.max_version, "1.0.0");

    let files = app.stored_files();
    assert_eq!(files, vec!["crates/foo_new/foo_new-1.0.0.crate"]);
}

#[test]
fn new_krate_weird_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_weird").version("0.0.0-pre");
    let json: GoodCrate = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_weird");
    assert_eq!(json.krate.max_version, "0.0.0-pre");

    let files = app.stored_files();
    assert_eq!(files, vec!["crates/foo_weird/foo_weird-0.0.0-pre.crate"]);
}

#[test]
fn new_with_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("package-name").rename("my-name");

    let crate_to_publish = PublishBuilder::new("new-krate")
        .version("1.0.0")
        .dependency(dependency);
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
fn new_with_underscore_renamed_dependency() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that new-krate can depend on it
        CrateBuilder::new("package-name", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("package-name").rename("_my-name");

    let crate_to_publish = PublishBuilder::new("new-krate")
        .version("1.0.0")
        .dependency(dependency);
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

    let crate_to_publish = PublishBuilder::new("new_dep")
        .version("1.0.0")
        .dependency(dependency);

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

    let crate_to_publish = PublishBuilder::new("new_dep")
        .version("1.0.0")
        .dependency(dependency);

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

    let crate_to_publish = PublishBuilder::new("new_dep")
        .version("1.0.0")
        .dependency(dependency);

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
    let crate_to_publish = PublishBuilder::new("foo").dependency(dependency);
    token.publish_crate(crate_to_publish).good();
}

#[test]
fn reject_new_crate_with_alternative_registry_dependency() {
    let (app, _, _, token) = TestApp::full().with_token();

    let dependency =
        DependencyBuilder::new("dep").registry("https://server.example/path/to/registry");

    let crate_to_publish = PublishBuilder::new("depends-on-alt-registry").dependency(dependency);
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

    let crate_to_publish = PublishBuilder::new("new_wild")
        .version("1.0.0")
        .dependency(dependency);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": WILDCARD_ERROR_MESSAGE }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_twice() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_twice").version("0.99.0");
    token.publish_crate(crate_to_publish).good();

    let crate_to_publish = PublishBuilder::new("foo_twice")
        .version("2.0.0")
        .description("2.0.0 description");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_twice");
    assert_eq!(json.krate.description.unwrap(), "2.0.0 description");

    let crates = app.crates_from_index_head("foo_twice");
    assert_eq!(crates.len(), 2);
    assert_eq!(crates[0].name, "foo_twice");
    assert_eq!(crates[0].vers, "0.99.0");
    assert!(crates[0].deps.is_empty());
    assert_eq!(crates[1].name, "foo_twice");
    assert_eq!(crates[1].vers, "2.0.0");
    assert!(crates[1].deps.is_empty());

    assert_eq!(
        app.stored_files(),
        vec![
            "crates/foo_twice/foo_twice-0.99.0.crate",
            "crates/foo_twice/foo_twice-2.0.0.crate"
        ]
    );
}

#[test]
fn new_krate_wrong_user() {
    let (app, _, user) = TestApp::full().with_user();

    app.db(|conn| {
        // Create the foo_wrong crate with one user
        CrateBuilder::new("foo_wrong", user.as_model().id).expect_build(conn);
    });

    // Then try to publish with a different user
    let another_user = app.db_new_user("another").db_new_token("bar");
    let crate_to_publish = PublishBuilder::new("foo_wrong").version("2.0.0");

    let response = another_user.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": MISSING_RIGHTS_ERROR_MESSAGE }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_too_big() {
    let (app, _, user) = TestApp::full().with_user();

    let files = [("foo_big-1.0.0/big", &[b'a'; 2000] as &[_])];
    let builder = PublishBuilder::new("foo_big").files(&files);

    let response = user.publish_crate(builder);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "uploaded tarball is malformed or too large when decompressed" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_too_big_but_whitelisted() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_whitelist", user.as_model().id)
            .max_upload_size(2_000_000)
            .expect_build(conn);
    });

    let files = [("foo_whitelist-1.1.0/big", &[b'a'; 2000] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_whitelist")
        .version("1.1.0")
        .files(&files);

    token.publish_crate(crate_to_publish).good();

    let files = app.stored_files();
    assert_eq!(
        files,
        vec!["crates/foo_whitelist/foo_whitelist-1.1.0.crate"]
    );
}

#[test]
fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user();
    let data: &[u8] = &[1];
    let files = [("foo-1.0.0/a", data), ("bar-1.0.0/a", data)];
    let builder = PublishBuilder::new("foo").files(&files);

    let response = user.publish_crate(builder);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid path found: bar-1.0.0/a" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_gzip_bomb() {
    let (app, _, _, token) = TestApp::full().with_token();

    let len = 512 * 1024;
    let mut body = io::repeat(0).take(len);

    let crate_to_publish = PublishBuilder::new("foo")
        .version("1.1.0")
        .files_with_io(&mut [("foo-1.1.0/a", &mut body, len)]);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "uploaded tarball is malformed or too large when decompressed" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_duplicate_version() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database and then we'll try to publish the same version
        CrateBuilder::new("foo_dupe", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_dupe").version("1.0.0");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "crate version `1.0.0` is already uploaded" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_crate_similar_name() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("Foo_similar", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_similar").version("1.1.0");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "crate was previously named `Foo_similar`" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_crate_similar_name_hyphen() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_bar_hyphen", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo-bar-hyphen").version("1.1.0");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "crate was previously named `foo_bar_hyphen`" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_crate_similar_name_underscore() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo-bar-underscore", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
    });

    let crate_to_publish = PublishBuilder::new("foo_bar_underscore").version("1.1.0");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "crate was previously named `foo-bar-underscore`" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_git_upload_with_conflicts() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.upstream_index().create_empty_commit().unwrap();

    let crate_to_publish = PublishBuilder::new("foo_conflicts");
    token.publish_crate(crate_to_publish).good();

    assert_eq!(
        app.stored_files(),
        vec!["crates/foo_conflicts/foo_conflicts-1.0.0.crate"]
    );
}

#[test]
fn new_krate_dependency_missing() {
    let (app, _, _, token) = TestApp::full().with_token();

    // Deliberately not inserting this crate in the database to test behavior when a dependency
    // doesn't exist!
    let dependency = DependencyBuilder::new("bar_missing");
    let crate_to_publish = PublishBuilder::new("foo_missing").dependency(dependency);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "no known crate named `bar_missing`" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_with_readme() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme").readme("");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0");

    let files = app.stored_files();
    assert_eq!(files, vec!["crates/foo_readme/foo_readme-1.0.0.crate",]);
}

#[test]
fn new_krate_with_readme_and_plus_version() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_readme")
        .version("1.0.0+foo")
        .readme("");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_readme");
    assert_eq!(json.krate.max_version, "1.0.0+foo");

    assert_eq!(
        app.stored_files(),
        vec![
            "crates/foo_readme/foo_readme-1.0.0 foo.crate",
            "crates/foo_readme/foo_readme-1.0.0+foo.crate"
        ]
    );
}

#[test]
fn new_krate_without_any_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        delete(emails::table).execute(conn).unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_no_email");

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address." }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_with_unverified_email_fails() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        update(emails::table)
            .set((emails::verified.eq(false),))
            .execute(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_unverified_email");

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "A verified email address is required to publish crates to crates.io. Visit https://crates.io/settings/profile to set and verify your email address." }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn publish_records_an_audit_action() {
    use crates_io::models::VersionOwnerAction;

    let (app, anon, _, token) = TestApp::full().with_token();

    app.db(|conn| assert!(VersionOwnerAction::all(conn).unwrap().is_empty()));

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.publish_crate(crate_to_publish).good();

    // Make sure it has one publish audit action
    let json = anon.show_version("fyk", "1.0.0");
    let actions = json.version.audit_actions;

    assert_eq!(actions.len(), 1);
    let action = &actions[0];
    assert_eq!(action.action, "publish");
    assert_eq!(action.user.id, token.as_model().user_id);
}

#[test]
fn publish_after_removing_documentation() {
    let (app, anon, user, token) = TestApp::full().with_token();
    let user = user.as_model();

    // 1. Start with a crate with no documentation
    app.db(|conn| {
        CrateBuilder::new("docscrate", user.id)
            .version("0.2.0")
            .expect_build(conn);
    });

    // Verify that crates start without any documentation so the next assertion can *prove*
    // that it was the one that added the documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, None);

    // 2. Add documentation
    let crate_to_publish = PublishBuilder::new("docscrate")
        .version("0.2.1")
        .documentation("http://foo.rs");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // Ensure latest version also has the same documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, Some("http://foo.rs".to_owned()));

    // 3. Remove the documentation
    let crate_to_publish = PublishBuilder::new("docscrate").version("0.2.2");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.documentation, None);

    // Ensure latest version no longer has documentation
    let json = anon.show_crate("docscrate");
    assert_eq!(json.krate.documentation, None);
}

#[test]
fn good_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish = PublishBuilder::new("foo_good_key")
        .keyword("c++")
        .keyword("crates-io_index")
        .keyword("1password");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.name, "foo_good_key");
    assert_eq!(json.krate.max_version, "1.0.0");
}

#[test]
fn bad_keywords() {
    let (_, _, _, token) = TestApp::full().with_token();
    let crate_to_publish =
        PublishBuilder::new("foo_bad_key").keyword("super-long-keyword-name-oh-no");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid length 29, expected a keyword with less than 20 characters at line 1 column 203" }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_bad_key").keyword("?@?%");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid value: string \"?@?%\", expected a valid keyword specifier at line 1 column 178" }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_bad_key").keyword("áccênts");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid upload request: invalid value: string \"áccênts\", expected a valid keyword specifier at line 1 column 183" }] })
    );
}

#[test]
fn good_categories() {
    let (app, _, _, token) = TestApp::full().with_token();

    app.db(|conn| {
        new_category("Category 1", "cat1", "Category 1 crates")
            .create_or_update(conn)
            .unwrap();
    });

    let crate_to_publish = PublishBuilder::new("foo_good_cat").category("cat1");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_good_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories.len(), 0);
}

#[test]
fn ignored_categories() {
    let (_, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_ignored_cat").category("bar");
    let json = token.publish_crate(crate_to_publish).good();

    assert_eq!(json.krate.name, "foo_ignored_cat");
    assert_eq!(json.krate.max_version, "1.0.0");
    assert_eq!(json.warnings.invalid_categories, vec!["bar"]);
}

#[test]
fn license_and_description_required() {
    let (app, _, _, token) = TestApp::full().with_token();

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_license()
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description", "license"]) }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description"]) }] })
    );

    let crate_to_publish = PublishBuilder::new("foo_metadata")
        .version("1.1.0")
        .unset_license()
        .license_file("foo")
        .unset_description();

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": missing_metadata_error_message(&["description"]) }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_tarball_with_hard_links() {
    let (app, _, _, token) = TestApp::full().with_token();

    let mut tarball = Vec::new();
    {
        let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, Compression::default()));
        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/bar"));
        header.set_size(0);
        header.set_entry_type(tar::EntryType::hard_link());
        assert_ok!(header.set_link_name("foo-1.1.0/another"));
        header.set_cksum();
        assert_ok!(ar.append(&header, &[][..]));
        assert_ok!(ar.finish());
    }

    let crate_to_publish = PublishBuilder::new("foo").version("1.1.0").tarball(tarball);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "unexpected symlink or hard link found: foo-1.1.0/bar" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn tarball_between_default_axum_limit_and_max_upload_size() {
    // We explicitly disable compression to be able to influence the final tarball size
    let compression = Compression::none();

    let max_upload_size = 5 * 1024 * 1024;
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = max_upload_size;
            config.max_unpack_size = max_upload_size;
        })
        .with_token();

    let mut tarball = Vec::new();
    {
        // `data` is smaller than `max_upload_size`, but bigger than the regular request body limit
        let data = &[b'a'; 3 * 1024 * 1024] as &[_];

        let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, compression));
        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/Cargo.toml"));
        header.set_size(data.len() as u64);
        header.set_cksum();
        assert_ok!(ar.append(&header, data));
        assert_ok!(ar.finish());
    }

    let crate_to_publish = PublishBuilder::new("foo").version("1.1.0").tarball(tarball);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    let json = response.good();
    assert_eq!(json.krate.name, "foo");
    assert_eq!(json.krate.max_version, "1.1.0");

    assert_eq!(app.stored_files().len(), 1);
}

#[test]
fn tarball_bigger_than_max_upload_size() {
    // We explicitly disable compression to be able to influence the final tarball size
    let compression = Compression::none();

    let max_upload_size = 5 * 1024 * 1024;
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_upload_size = max_upload_size;
            config.max_unpack_size = max_upload_size;
        })
        .with_token();

    let mut tarball = Vec::new();
    {
        // `data` is bigger than `max_upload_size`
        let data = &[b'a'; 6 * 1024 * 1024] as &[_];

        let mut ar = tar::Builder::new(GzEncoder::new(&mut tarball, compression));
        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/Cargo.toml"));
        header.set_size(data.len() as u64);
        header.set_cksum();
        assert_ok!(ar.append(&header, data));
        assert_ok!(ar.finish());
    }

    let crate_to_publish = PublishBuilder::new("foo").version("1.1.0").tarball(tarball);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": format!("max upload size is: {max_upload_size}") }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn publish_new_crate_rate_limited() {
    let (app, anon, _, token) = TestApp::full()
        .with_publish_rate_limit(Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1");
    token.publish_crate(crate_to_publish).good();

    assert_eq!(app.stored_files().len(), 1);

    // Uploading a second crate is limited
    let crate_to_publish = PublishBuilder::new("rate_limited2");
    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

    assert_eq!(app.stored_files().len(), 1);

    let response = anon.get::<()>("/api/v1/crates/rate_limited2");
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    // Wait for the limit to be up
    thread::sleep(Duration::from_millis(500));

    let crate_to_publish = PublishBuilder::new("rate_limited2");
    token.publish_crate(crate_to_publish).good();

    let json = anon.show_crate("rate_limited2");
    assert_eq!(json.krate.max_version, "1.0.0");

    assert_eq!(app.stored_files().len(), 2);
}

#[test]
fn publish_rate_limit_doesnt_affect_existing_crates() {
    let (_, _, _, token) = TestApp::full()
        .with_publish_rate_limit(Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("rate_limited1");
    token.publish_crate(crate_to_publish).good();

    let new_version = PublishBuilder::new("rate_limited1").version("1.0.1");
    token.publish_crate(new_version).good();
}

#[test]
fn features_version_2() {
    let (app, _, user, token) = TestApp::full().with_token();

    app.db(|conn| {
        // Insert a crate directly into the database so that foo_new can depend on it
        CrateBuilder::new("bar", user.as_model().id).expect_build(conn);
    });

    let dependency = DependencyBuilder::new("bar");

    let crate_to_publish = PublishBuilder::new("foo")
        .version("1.0.0")
        .dependency(dependency)
        .feature("new_feat", &["dep:bar", "bar?/feat"])
        .feature("old_feat", &[]);
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("foo");
    assert_eq!(crates.len(), 1);
    assert_eq!(crates[0].name, "foo");
    assert_eq!(crates[0].deps.len(), 1);
    assert_eq!(crates[0].v, Some(2));
    let features = BTreeMap::from_iter([("old_feat".to_string(), vec![])]);
    assert_eq!(crates[0].features, features);
    let features2 = BTreeMap::from_iter([(
        "new_feat".to_string(),
        vec!["dep:bar".to_string(), "bar?/feat".to_string()],
    )]);
    assert_eq!(crates[0].features2, Some(features2));
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
    let crate_to_publish = PublishBuilder::new("two-deps")
        .version("1.0.0")
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

#[test]
fn empty_payload() {
    let (app, _, user) = TestApp::full().with_user();

    let response = user.put::<()>("/api/v1/crates/new", &[]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid metadata length" }] })
    );

    assert!(app.stored_files().is_empty());
}

fn version_with_build_metadata(v1: &str, v2: &str, expected_error: &str) {
    let (_app, _anon, _cookie, token) = TestApp::full().with_token();

    let response = token.publish_crate(PublishBuilder::new("foo").version(v1));
    assert_eq!(response.status(), StatusCode::OK);
    assert_yaml_snapshot!(response.into_json(), {
        ".crate.created_at" => "[datetime]",
        ".crate.updated_at" => "[datetime]",
    });

    let response = token.publish_crate(PublishBuilder::new("foo").version(v2));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": expected_error }] })
    );
}

#[test]
fn version_with_build_metadata_1() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_1" }, {
        version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0+bar",
            "crate version `1.0.0` is already uploaded",
        );
    });
}

#[test]
fn version_with_build_metadata_2() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_2" }, {
        version_with_build_metadata(
            "1.0.0-beta.1",
            "1.0.0-beta.1+2",
            "crate version `1.0.0-beta.1` is already uploaded",
        );
    });
}

#[test]
fn version_with_build_metadata_3() {
    insta::with_settings!({ snapshot_suffix => "build_metadata_3" }, {
        version_with_build_metadata(
            "1.0.0+foo",
            "1.0.0",
            "crate version `1.0.0` is already uploaded",
        );
    });
}
