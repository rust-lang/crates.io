use crate::{
    add_team_to_crate,
    builders::{CrateBuilder, PublishBuilder},
    new_team,
    record::GhUser,
    OwnerTeamsResponse, RequestHelper, TestApp,
};
use cargo_registry::models::{Crate, NewTeam, NewUser};
use std::sync::Once;

use conduit::StatusCode;
use diesel::*;

impl crate::util::MockAnonymousUser {
    /// List the team owners of the specified crate.
    fn crate_owner_teams(&self, krate_name: &str) -> crate::util::Response<OwnerTeamsResponse> {
        let url = format!("/api/v1/crates/{krate_name}/owner_team");
        self.get(&url)
    }
}

// Users: `crates-tester-1` and `crates-tester-2`
// Passwords: ask acrichto or gankro
// Teams: `crates-test-org:core`, `crates-test-org:just-for-crates-2`
// tester-1 is on core only, tester-2 is on both

static GH_USER_1: GhUser = GhUser {
    login: "crates-tester-1",
    init: Once::new(),
};
static GH_USER_2: GhUser = GhUser {
    login: "crates-tester-2",
    init: Once::new(),
};

fn mock_user_on_only_one_team() -> NewUser<'static> {
    GH_USER_1.user()
}
fn mock_user_on_both_teams() -> NewUser<'static> {
    GH_USER_2.user()
}

// Test adding team without `github:`
#[test]
fn not_github() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_not_github", user.as_model().id).expect_build(conn);
    });

    let response = token.add_named_owner("foo_not_github", "dropbox:foo:foo");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "unknown organization handler, only 'github:org:team' is supported" }] })
    );
}

#[test]
fn weird_name() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_weird_name", user.as_model().id).expect_build(conn);
    });

    let response = token.add_named_owner("foo_weird_name", "github:foo/../bar:wut");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "organization cannot contain special characters like /" }] })
    );
}

// Test adding team without second `:`
#[test]
fn one_colon() {
    let (app, _, user, token) = TestApp::init().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_one_colon", user.as_model().id).expect_build(conn);
    });

    let response = token.add_named_owner("foo_one_colon", "github:foo");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "missing github team argument; format is github:org:team" }] })
    );
}

#[test]
fn nonexistent_team() {
    let (app, _, user, token) = TestApp::with_proxy().with_token();

    app.db(|conn| {
        CrateBuilder::new("foo_nonexistent", user.as_model().id).expect_build(conn);
    });

    let response = token.add_named_owner(
        "foo_nonexistent",
        "github:crates-test-org:this-does-not-exist",
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "could not find the github team crates-test-org/this-does-not-exist" }] })
    );
}

// Test adding a renamed team
#[test]
fn add_renamed_team() {
    let (app, anon, user, token) = TestApp::with_proxy().with_token();
    let owner_id = user.as_model().id;

    app.db(|conn| {
        use cargo_registry::schema::teams::dsl::*;

        CrateBuilder::new("foo_renamed_team", owner_id).expect_build(conn);

        // create team with same ID and different name compared to http mock
        // used for `add_named_owner`
        NewTeam::new(
            "github:crates-test-org:not_core", // different team name
            13804222,                          // same org ID
            1699377,                           // same team id as `core` team
            None,
            None,
        )
        .create_or_update(conn)
        .unwrap();

        assert_eq!(teams.count().get_result::<i64>(conn).unwrap(), 1);
    });

    token
        .add_named_owner("foo_renamed_team", "github:crates-test-org:core")
        .good();

    let json = anon.crate_owner_teams("foo_renamed_team").good();
    assert_eq!(json.teams.len(), 1);
    assert_eq!(json.teams[0].login, "github:crates-test-org:core");
}

// Test adding team names with mixed case, when on the team
#[test]
fn add_team_mixed_case() {
    let (app, anon) = TestApp::with_proxy().empty();
    let user = app.db_new_user(mock_user_on_both_teams().gh_login);
    let token = user.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_mixed_case", user.as_model().id).expect_build(conn);
    });

    token
        .add_named_owner("foo_mixed_case", "github:Crates-Test-Org:Core")
        .good();

    app.db(|conn| {
        let krate: Crate = Crate::by_name("foo_mixed_case").first(conn).unwrap();
        let owners = krate.owners(conn).unwrap();
        assert_eq!(owners.len(), 2);
        let owner = &owners[1];
        assert_eq!(owner.login(), owner.login().to_lowercase());
    });

    let json = anon.crate_owner_teams("foo_mixed_case").good();
    assert_eq!(json.teams.len(), 1);
    assert_eq!(json.teams[0].login, "github:crates-test-org:core");
}

// Test adding team as owner when not on it
#[test]
fn add_team_as_non_member() {
    let (app, _) = TestApp::with_proxy().empty();
    let user = app.db_new_user(mock_user_on_only_one_team().gh_login);
    let token = user.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_team_non_member", user.as_model().id).expect_build(conn);
    });

    let response = token.add_named_owner(
        "foo_team_non_member",
        "github:crates-test-org:just-for-crates-2",
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "only members of a team can add it as an owner" }] })
    );
}

#[test]
fn remove_team_as_named_owner() {
    let (app, _) = TestApp::with_proxy().empty();
    let username = mock_user_on_both_teams().gh_login;
    let user_on_both_teams = app.db_new_user(username);
    let token_on_both_teams = user_on_both_teams.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_remove_team", user_on_both_teams.as_model().id).expect_build(conn);
    });

    token_on_both_teams
        .add_named_owner("foo_remove_team", "github:crates-test-org:core")
        .good();

    // Removing the individual owner is not allowed, since team members don't
    // have permission to manage ownership
    let response = token_on_both_teams.remove_named_owner("foo_remove_team", username);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "cannot remove all individual owners of a crate. Team member don't have permission to modify owners, so at least one individual owner is required." }] })
    );

    token_on_both_teams
        .remove_named_owner("foo_remove_team", "github:crates-test-org:core")
        .good();

    let user_on_one_team = app.db_new_user(mock_user_on_only_one_team().gh_login);
    let crate_to_publish = PublishBuilder::new("foo_remove_team").version("2.0.0");
    let response = user_on_one_team.enqueue_publish(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing." }] })
    );
}

#[test]
fn remove_team_as_team_owner() {
    let (app, _) = TestApp::with_proxy().empty();
    let user_on_both_teams = app.db_new_user(mock_user_on_both_teams().gh_login);
    let token_on_both_teams = user_on_both_teams.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_remove_team_owner", user_on_both_teams.as_model().id)
            .expect_build(conn);
    });

    token_on_both_teams
        .add_named_owner("foo_remove_team_owner", "github:crates-test-org:core")
        .good();

    let user_on_one_team = app.db_new_user(mock_user_on_only_one_team().gh_login);
    let token_on_one_team = user_on_one_team.db_new_token("arbitrary token name");

    let response = token_on_one_team
        .remove_named_owner("foo_remove_team_owner", "github:crates-test-org:core");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "team members don't have permission to modify owners" }] })
    );
}

// Test trying to publish a crate we don't own
#[test]
fn publish_not_owned() {
    let (app, _) = TestApp::with_proxy().empty();
    let user_on_both_teams = app.db_new_user(mock_user_on_both_teams().gh_login);
    let token_on_both_teams = user_on_both_teams.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_not_owned", user_on_both_teams.as_model().id).expect_build(conn);
    });

    token_on_both_teams
        .add_named_owner("foo_not_owned", "github:crates-test-org:just-for-crates-2")
        .good();

    let user_on_one_team = app.db_new_user(mock_user_on_only_one_team().gh_login);

    let crate_to_publish = PublishBuilder::new("foo_not_owned").version("2.0.0");
    let response = user_on_one_team.enqueue_publish(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "this crate exists but you don't seem to be an owner. If you believe this is a mistake, perhaps you need to accept an invitation to be an owner before publishing." }] })
    );
}

// Test trying to publish a krate we do own (but only because of teams)
#[test]
fn publish_owned() {
    let (app, _) = TestApp::full().empty();
    let user_on_both_teams = app.db_new_user(mock_user_on_both_teams().gh_login);
    let token_on_both_teams = user_on_both_teams.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_team_owned", user_on_both_teams.as_model().id).expect_build(conn);
    });

    token_on_both_teams
        .add_named_owner("foo_team_owned", "github:crates-test-org:core")
        .good();

    let user_on_one_team = app.db_new_user(mock_user_on_only_one_team().gh_login);

    let crate_to_publish = PublishBuilder::new("foo_team_owned").version("2.0.0");
    user_on_one_team.enqueue_publish(crate_to_publish).good();
}

// Test trying to change owners (when only on an owning team)
#[test]
fn add_owners_as_team_owner() {
    let (app, _) = TestApp::with_proxy().empty();
    let user_on_both_teams = app.db_new_user(mock_user_on_both_teams().gh_login);
    let token_on_both_teams = user_on_both_teams.db_new_token("arbitrary token name");

    app.db(|conn| {
        CrateBuilder::new("foo_add_owner", user_on_both_teams.as_model().id).expect_build(conn);
    });

    token_on_both_teams
        .add_named_owner("foo_add_owner", "github:crates-test-org:core")
        .good();

    let user_on_one_team = app.db_new_user(mock_user_on_only_one_team().gh_login);
    let token_on_one_team = user_on_one_team.db_new_token("arbitrary token name");

    let response = token_on_one_team.add_named_owner("foo_add_owner", "arbitrary_username");
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "team members don't have permission to modify owners" }] })
    );
}

#[test]
fn crates_by_team_id() {
    let (app, anon, user) = TestApp::init().with_user();
    let user = user.as_model();

    let team = app.db(|conn| {
        let t = new_team("github:crates-test-org:team_foo")
            .create_or_update(conn)
            .unwrap();
        let krate = CrateBuilder::new("foo", user.id).expect_build(conn);
        add_team_to_crate(&t, &krate, user, conn).unwrap();
        t
    });

    let json = anon.search(&format!("team_id={}", team.id));
    assert_eq!(json.crates.len(), 1);
}

#[test]
fn crates_by_team_id_not_including_deleted_owners() {
    // This needs to use the proxy beacuse removing a team checks with github that you're on the
    // team before you're allowed to remove it from the crate
    let (app, anon) = TestApp::with_proxy().empty();
    let user = app.db_new_user(mock_user_on_both_teams().gh_login);
    let user = user.as_model();

    let team = app.db(|conn| {
        let t = NewTeam::new("github:crates-test-org:core", 13804222, 1699377, None, None)
            .create_or_update(conn)
            .unwrap();

        let krate = CrateBuilder::new("foo", user.id).expect_build(conn);
        add_team_to_crate(&t, &krate, user, conn).unwrap();
        krate
            .owner_remove(app.as_inner(), conn, user, &t.login)
            .unwrap();
        t
    });

    let json = anon.search(&format!("team_id={}", team.id));
    assert_eq!(json.crates.len(), 0);
}
