use crate::builders::{CrateBuilder, UserBuilder};
use crate::owners::expire_invitation;
use crate::util::github::next_gh_id;
use crate::util::{RequestHelper, Response, TestApp};
use crate::{OwnerResp, new_team};
use crates_io::models::CrateOwner;
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::schema::{crate_owner_invitations, emails, teams};
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_snapshot;

// This is testing Cargo functionality! ! !
// specifically functions modify_owners and add_owners
// which call the `PUT /crates/{crate_id}/owners` route
#[tokio::test(flavor = "multi_thread")]
async fn test_cargo_invite_owners() {
    let (app, _, owner) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let new_user = app.db_new_user("cilantro").await;
    CrateBuilder::new("guacamole", owner.as_model().id)
        .expect_build(&mut conn)
        .await;

    let json = owner
        .add_named_owner("guacamole", &new_user.as_model().username)
        .await
        .good();

    // this ok:true field is what old versions of Cargo
    // need - do not remove unless you're cool with
    // dropping support for old versions
    assert!(json.ok);
    // msg field is what is sent and used in updated
    // version of cargo
    assert_eq!(
        json.msg,
        "Crates.io user cilantro has been invited to be an owner of crate guacamole"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_cookie() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

/// Reused GitHub logins select the highest GitHub ID, regardless of crates.io ID.
#[tokio::test(flavor = "multi_thread")]
async fn reused_github_login_invites_highest_github_id() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let older_gh_id = next_gh_id();
    let newer = UserBuilder::new()
        .with_username("newer")
        .with_gh_login("SHARED");
    let newer = app.db_new_user_from_builder(newer).await;
    let older = UserBuilder::new()
        .with_username("older")
        .with_gh_login("shared")
        .with_gh_id(older_gh_id);
    let older = app.db_new_user_from_builder(older).await;

    assert!(newer.as_model().gh_id > older.as_model().gh_id);
    assert!(newer.as_model().id < older.as_model().id);

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.add_named_owner("foo", "shared").await;
    assert_snapshot!(response.status(), @"200 OK");

    let invitees = invited_user_ids(&mut conn, krate.id).await;
    assert_eq!(invitees, [newer.as_model().id]);
}

/// An older owner's cached GitHub login does not block inviting a different user.
#[tokio::test(flavor = "multi_thread")]
async fn reused_github_login_invites_newer_account_when_older_owns_crate() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let older = UserBuilder::new()
        .with_username("older")
        .with_gh_login("shared");
    let older = app.db_new_user_from_builder(older).await;
    let newer = UserBuilder::new()
        .with_username("newer")
        .with_gh_login("SHARED");
    let newer = app.db_new_user_from_builder(newer).await;
    assert!(newer.as_model().gh_id > older.as_model().gh_id);

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;
    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(older.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.add_named_owner("foo", "SHARED").await;
    assert_snapshot!(response.status(), @"200 OK");

    let invitees = invited_user_ids(&mut conn, krate.id).await;
    assert_eq!(invitees, [newer.as_model().id]);
}

/// An owner excluded from user lookup produces a lookup error, not a duplicate error.
#[tokio::test(flavor = "multi_thread")]
async fn owner_lookup_error_precedes_duplicate_check() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let inactive = UserBuilder::new().with_username("inactive").with_gh_id(-1);
    let inactive = app.db_new_user_from_builder(inactive).await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;
    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(inactive.as_model().id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.add_named_owner("foo", "INACTIVE").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `INACTIVE`"}]}"#);
    assert!(invited_user_ids(&mut conn, krate.id).await.is_empty());
}

/// User and team IDs belong to separate namespaces.
#[tokio::test(flavor = "multi_thread")]
async fn invite_user_with_same_id_as_team_owner() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;
    let invitee = app.db_new_user("invitee").await;
    let invitee_id = invitee.as_model().id;

    diesel::insert_into(teams::table)
        .values((teams::id.eq(invitee_id), new_team("github:org:team")))
        .execute(&mut conn)
        .await
        .unwrap();

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;
    CrateOwner::builder()
        .crate_id(krate.id)
        .team_id(invitee_id)
        .created_by(cookie.as_model().id)
        .build()
        .insert(&conn)
        .await
        .unwrap();

    let response = cookie.add_named_owner("foo", "invitee").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_eq!(invited_user_ids(&mut conn, krate.id).await, [invitee_id]);
}

/// Returns the invited user IDs for a crate in ascending order.
async fn invited_user_ids(conn: &mut AsyncPgConnection, crate_id: i32) -> Vec<i32> {
    crate_owner_invitations::table
        .filter(crate_owner_invitations::crate_id.eq(crate_id))
        .select(crate_owner_invitations::invited_user_id)
        .order(crate_owner_invitations::invited_user_id)
        .load(conn)
        .await
        .unwrap()
}

async fn invite_distinct_login_user(login: &str) -> Response<OwnerResp> {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user = UserBuilder::new()
        .with_username("crates-user")
        .with_gh_login("github-user");

    app.db_new_user_from_builder(user).await;

    let krate = CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    cookie.add_named_owner(&krate.name, login).await
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_verbatim() {
    let response = invite_distinct_login_user("crates-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `crates-user`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_case_insensitive() {
    let response = invite_distinct_login_user("CRATES-USER").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `CRATES-USER`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_crates_io_username_separator_variant() {
    let response = invite_distinct_login_user("crates_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `crates_user`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_verbatim() {
    let response = invite_distinct_login_user("github-user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user crates-user has been invited to be an owner of crate foo","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_case_insensitive() {
    let response = invite_distinct_login_user("GITHUB-USER").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user crates-user has been invited to be an owner of crate foo","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn unprefixed_github_login_separator_variant() {
    let response = invite_distinct_login_user("github_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `github_user`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_verbatim() {
    let response = invite_distinct_login_user("crates.io:crates-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown organization handler, only 'github:org:team' is supported"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_case_insensitive() {
    let response = invite_distinct_login_user("crates.io:CRATES-USER").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown organization handler, only 'github:org:team' is supported"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefixed_username_separator_variant() {
    let response = invite_distinct_login_user("crates.io:crates_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown organization handler, only 'github:org:team' is supported"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_verbatim() {
    let response = invite_distinct_login_user("github:github-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing github team argument; format is github:org:team"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_case_insensitive() {
    let response = invite_distinct_login_user("github:GITHUB-USER").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing github team argument; format is github:org:team"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefixed_login_separator_variant() {
    let response = invite_distinct_login_user("github:github_user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing github team argument; format is github:org:team"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crates_io_prefix_does_not_match_github_login() {
    let response = invite_distinct_login_user("crates.io:github-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"unknown organization handler, only 'github:org:team' is supported"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_prefix_does_not_match_crates_io_username() {
    let response = invite_distinct_login_user("github:crates-user").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing github team argument; format is github:org:team"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_token() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token() {
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::ChangeOwners]))
        .await;

    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token_with_matching_crate_scope() {
    let crate_scopes = Some(vec![CrateScope::try_from("foo_crate").unwrap()]);
    let endpoint_scopes = Some(vec![EndpointScope::ChangeOwners]);
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(crate_scopes, endpoint_scopes)
        .await;
    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token_with_wrong_crate_scope() {
    let crate_scopes = Some(vec![CrateScope::try_from("bar").unwrap()]);
    let endpoint_scopes = Some(vec![EndpointScope::ChangeOwners]);
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(crate_scopes, endpoint_scopes)
        .await;
    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_publish_token() {
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::PublishUpdate]))
        .await;

    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let response = token.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_without_auth() {
    let (app, anon, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = anon.add_named_owner(&krate.name, &user2.username).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_legacy_field() {
    let (app, _, user1) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user1.as_model().id)
        .expect_build(&mut conn)
        .await;
    app.db_new_user("user2").await;

    let input = r#"{"users": ["user2"]}"#;
    let response = user1
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user user2 has been invited to be an owner of crate foo","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_invalid_json() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    app.db_new_user("bar").await;
    CrateBuilder::new("foo", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    // incomplete input
    let input = r#"{"owners": ["foo", }"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: owners[1]: expected value at line 1 column 20"}]}"#);

    // `owners` is not an array
    let input = r#"{"owners": "foo"}"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: owners: invalid type: string \"foo\", expected a sequence at line 1 column 16"}]}"#);

    // missing `owners` and/or `users` fields
    let input = r#"{}"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `owners` at line 1 column 2"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invite_already_invited_user() {
    let (app, _, _, owner) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    app.db_new_user("invited_user").await;
    CrateBuilder::new("crate_name", owner.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(app.emails().await.len(), 1);

    // Then invite the user a second time, the message should point out the user is already invited
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user invited_user already has a pending invitation to be an owner of crate crate_name","ok":true}"#);

    // Check that no new email is sent after the second invitation
    assert_eq!(app.emails().await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn invite_with_existing_expired_invite() {
    let (app, _, _, owner) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    app.db_new_user("invited_user").await;
    let krate = CrateBuilder::new("crate_name", owner.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(app.emails().await.len(), 1);

    // Simulate the previous invite expiring
    expire_invitation(&app, krate.id).await;

    // Then invite the user a second time, a new invite is created as the old one expired
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_snapshot!(response.text(), @r#"{"msg":"Crates.io user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check that the email for the second invite was sent
    assert_eq!(app.emails().await.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user().await;
    app.db_new_user("bar").await;

    let response = user.add_named_owner("unknown", "bar").await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie.add_named_owner("foo", "unknown").await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_team() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", cookie.as_model().id)
        .expect_build(&mut conn)
        .await;

    let response = cookie
        .add_named_owner("foo", "github:unknown:unknown")
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find the github team unknown/unknown. Make sure that you have the right permissions in GitHub. See https://doc.rust-lang.org/cargo/reference/publishing.html#github-permissions"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn max_invites_per_request() {
    let (app, _, _, owner) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("crate_name", owner.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let usernames = (0..11)
        .map(|i| format!("user_{i}"))
        .collect::<Vec<String>>();

    // Populate enough users in the database to submit 11 invites at once.
    for user in &usernames {
        app.db_new_user(user).await;
    }

    let response = owner.add_named_owners("crate_name", &usernames).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"too many invites for this request - maximum 10"}]}"#);
}

/// Invalid recipient addresses reject the request without exposing the address.
#[tokio::test(flavor = "multi_thread")]
async fn invite_with_invalid_email() -> anyhow::Result<()> {
    let (app, _, owner) = TestApp::full().with_user().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", owner.as_model().id)
        .expect_build(&mut conn)
        .await;
    app.db_new_user("valid_user").await;
    let invitee = UserBuilder::new()
        .with_username("invalid_user")
        .with_gh_login("github_user");
    let invitee = app.db_new_user_from_builder(invitee).await;
    let email = emails::table.filter(emails::user_id.eq(invitee.as_model().id));
    diesel::update(email)
        .set(emails::email.eq("private address@example.com"))
        .execute(&mut conn)
        .await?;
    // The email-change trigger clears `verified`, so set it in a separate update.
    diesel::update(email)
        .set(emails::verified.eq(true))
        .execute(&mut conn)
        .await?;

    let response = owner
        .add_named_owners("foo", &["valid_user", "github_user"])
        .await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"user `invalid_user` has an invalid email address"}]}"#);

    assert!(app.emails().await.is_empty());
    Ok(())
}

/// Asserts that emails are only sent if the request succeeds.
#[tokio::test(flavor = "multi_thread")]
async fn no_invite_emails_for_txn_rollback() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    CrateBuilder::new("crate_name", token.as_model().user_id)
        .expect_build(&mut conn)
        .await;

    let mut usernames = (0..9).map(|i| format!("user_{i}")).collect::<Vec<String>>();

    // Populate enough users in the database to submit 9 good invites.
    for user in &usernames {
        app.db_new_user(user).await;
    }

    // Add an invalid username to the end of the invite list to cause the
    // request to fail.
    usernames.push("bananas".to_string());

    let response = token.add_named_owners("crate_name", &usernames).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `bananas`"}]}"#);

    // No emails should have been sent.
    assert_eq!(app.emails().await.len(), 0);

    // Remove the bad username.
    let _ = usernames.pop();

    let response = token.add_named_owners("crate_name", &usernames).await;
    assert_snapshot!(response.status(), @"200 OK");

    // 9 emails to the good invitees should have been sent.
    assert_eq!(app.emails().await.len(), 9);
}
