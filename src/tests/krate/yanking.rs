use crate::builders::PublishBuilder;
use crate::routes::crates::versions::yank_unyank::YankRequestHelper;
use crate::util::{RequestHelper, TestApp};

#[test]
#[allow(unknown_lints, clippy::bool_assert_comparison)] // for claim::assert_some_eq! with bool
fn yank_works_as_intended() {
    let (app, anon, cookie, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk");
    token.publish_crate(crate_to_publish).good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    // make sure it's not yanked
    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    token.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    token.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);

    // yank it
    cookie.yank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, true);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(json.version.yanked);

    // un-yank it
    cookie.unyank("fyk", "1.0.0").good();

    let crates = app.crates_from_index_head("fyk");
    assert_eq!(crates.len(), 1);
    assert_some_eq!(crates[0].yanked, false);

    let json = anon.show_version("fyk", "1.0.0");
    assert!(!json.version.yanked);
}

#[test]
fn yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.publish_crate(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // yank version 2.0.0
    token.yank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // unyank version 2.0.0
    token.unyank("fyk_max", "2.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn publish_after_yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max");
    token.publish_crate(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max").version("2.0.0");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}

#[test]
fn yank_permissions() {
    let (_, anon, _, user_token, _, admin_token) = TestApp::full().with_user_and_admin_tokens();

    // Let's create a crate owned by the regular user.
    let krate = PublishBuilder::new("fyk");
    user_token.publish_crate(krate).good();
    assert_max_version(&anon, "fyk", "1.0.0");

    // Yank and unyank as the owning user and as an admin. Both should succeed.
    for token in [&user_token, &admin_token] {
        token.yank("fyk", "1.0.0").good();
        assert_max_version(&anon, "fyk", "0.0.0");

        token.unyank("fyk", "1.0.0").good();
        assert_max_version(&anon, "fyk", "1.0.0");
    }

    // Yank and unyank as the anonymous user. Both should fail.
    anon.yank("fyk", "1.0.0").assert_forbidden();
    assert_max_version(&anon, "fyk", "1.0.0");
    user_token.yank("fyk", "1.0.0").good();

    anon.unyank("fyk", "1.0.0").assert_forbidden();
    assert_max_version(&anon, "fyk", "0.0.0");
    user_token.unyank("fyk", "1.0.0").good();
}

#[track_caller]
fn assert_max_version<T: RequestHelper>(token: &T, krate: &str, want: &str) {
    assert_eq!(token.show_crate(krate).krate.max_version, want);
}
