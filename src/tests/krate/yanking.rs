use crate::builders::PublishBuilder;
use crate::routes::crates::versions::yank_unyank::YankRequestHelper;
use crate::util::{RequestHelper, TestApp};
use crates_io::rate_limiter::LimitedAction;
use std::time::Duration;

#[test]
#[allow(unknown_lints, clippy::bool_assert_comparison)] // for claim::assert_some_eq! with bool
fn yank_works_as_intended() {
    let (app, anon, cookie, token) = TestApp::full().with_token();

    // Upload a new crate, putting it in the git index
    let crate_to_publish = PublishBuilder::new("fyk", "1.0.0");
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
fn test_yank_rate_limiting() {
    #[track_caller]
    fn check_yanked(app: &TestApp, is_yanked: bool) {
        let crates = app.crates_from_index_head("yankable");
        assert_eq!(crates.len(), 1);
        assert_some_eq!(crates[0].yanked, is_yanked);
    }

    let (app, _, _, token) = TestApp::full()
        .with_rate_limit(LimitedAction::YankUnyank, Duration::from_millis(500), 1)
        .with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("yankable", "1.0.0");
    token.publish_crate(crate_to_publish).good();
    check_yanked(&app, false);

    // Yank the crate
    token.yank("yankable", "1.0.0").good();
    check_yanked(&app, true);

    // Try unyanking the crate, will get rate limited
    token
        .unyank("yankable", "1.0.0")
        .assert_rate_limited(LimitedAction::YankUnyank);
    check_yanked(&app, true);

    // Let the rate limit refill.
    std::thread::sleep(Duration::from_millis(500));

    // Unyanking now works.
    token.unyank("yankable", "1.0.0").good();
    check_yanked(&app, false);

    // Yanking again will trigger the rate limit.
    token
        .yank("yankable", "1.0.0")
        .assert_rate_limited(LimitedAction::YankUnyank);
    check_yanked(&app, false);
}

#[test]
fn yank_max_version() {
    let (_, anon, _, token) = TestApp::full().with_token();

    // Upload a new crate
    let crate_to_publish = PublishBuilder::new("fyk_max", "1.0.0");
    token.publish_crate(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max", "2.0.0");
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
    let crate_to_publish = PublishBuilder::new("fyk_max", "1.0.0");
    token.publish_crate(crate_to_publish).good();

    // double check the max version
    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "1.0.0");

    // yank version 1.0.0
    token.yank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "0.0.0");

    // add version 2.0.0
    let crate_to_publish = PublishBuilder::new("fyk_max", "2.0.0");
    let json = token.publish_crate(crate_to_publish).good();
    assert_eq!(json.krate.max_version, "2.0.0");

    // unyank version 1.0.0
    token.unyank("fyk_max", "1.0.0").good();

    let json = anon.show_crate("fyk_max");
    assert_eq!(json.krate.max_version, "2.0.0");
}
