use crate::models::token::{CrateScope, EndpointScope};
use crate::tests::builders::CrateBuilder;
use crate::tests::owners::expire_invitation;
use crate::tests::util::{RequestHelper, TestApp};
use http::StatusCode;
use insta::assert_snapshot;

// This is testing Cargo functionality! ! !
// specifically functions modify_owners and add_owners
// which call the `PUT /crates/:crate_id/owners` route
#[tokio::test(flavor = "multi_thread")]
async fn test_cargo_invite_owners() {
    let (app, _, owner) = TestApp::init().with_user().await;
    let mut conn = app.db_conn();

    let new_user = app.db_new_user("cilantro").await;
    CrateBuilder::new("guacamole", owner.as_model().id).expect_build(&mut conn);

    let json = owner
        .add_named_owner("guacamole", &new_user.as_model().gh_login)
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
        "user cilantro has been invited to be an owner of crate guacamole"
    )
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_cookie() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", cookie.as_model().id).expect_build(&mut conn);

    let response = cookie.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_token() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id).expect_build(&mut conn);

    let response = token.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token() {
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::ChangeOwners]))
        .await;

    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id).expect_build(&mut conn);

    let response = token.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token_with_matching_crate_scope() {
    let crate_scopes = Some(vec![CrateScope::try_from("foo_crate").unwrap()]);
    let endpoint_scopes = Some(vec![EndpointScope::ChangeOwners]);
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(crate_scopes, endpoint_scopes)
        .await;
    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id).expect_build(&mut conn);

    let response = token.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user user-2 has been invited to be an owner of crate foo_crate","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_change_owner_token_with_wrong_crate_scope() {
    let crate_scopes = Some(vec![CrateScope::try_from("bar").unwrap()]);
    let endpoint_scopes = Some(vec![EndpointScope::ChangeOwners]);
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(crate_scopes, endpoint_scopes)
        .await;
    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id).expect_build(&mut conn);

    let response = token.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_via_publish_token() {
    let (app, _, _, token) = TestApp::full()
        .with_scoped_token(None, Some(vec![EndpointScope::PublishUpdate]))
        .await;

    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", token.as_model().user_id).expect_build(&mut conn);

    let response = token.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_change_without_auth() {
    let (app, anon, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    let user2 = app.db_new_user("user-2").await;
    let user2 = user2.as_model();

    let krate = CrateBuilder::new("foo_crate", cookie.as_model().id).expect_build(&mut conn);

    let response = anon.add_named_owner(&krate.name, &user2.gh_login).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_legacy_field() {
    let (app, _, user1) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("foo", user1.as_model().id).expect_build(&mut conn);
    app.db_new_user("user2").await;

    let input = r#"{"users": ["user2"]}"#;
    let response = user1
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user user2 has been invited to be an owner of crate foo","ok":true}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_owner_change_with_invalid_json() {
    let (app, _, user) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    app.db_new_user("bar").await;
    CrateBuilder::new("foo", user.as_model().id).expect_build(&mut conn);

    // incomplete input
    let input = r#"{"owners": ["foo", }"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to parse the request body as JSON: owners[1]: expected value at line 1 column 20"}]}"#);

    // `owners` is not an array
    let input = r#"{"owners": "foo"}"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: owners: invalid type: string \"foo\", expected a sequence at line 1 column 16"}]}"#);

    // missing `owners` and/or `users` fields
    let input = r#"{}"#;
    let response = user
        .put::<()>("/api/v1/crates/foo/owners", input.as_bytes())
        .await;
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: missing field `owners` at line 1 column 2"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn invite_already_invited_user() {
    let (app, _, _, owner) = TestApp::init().with_token().await;
    let mut conn = app.db_conn();

    app.db_new_user("invited_user").await;
    CrateBuilder::new("crate_name", owner.as_model().user_id).expect_build(&mut conn);

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(app.emails().await.len(), 1);

    // Then invite the user a second time, the message should point out the user is already invited
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user invited_user already has a pending invitation to be an owner of crate crate_name","ok":true}"#);

    // Check that no new email is sent after the second invitation
    assert_eq!(app.emails().await.len(), 1);
}

#[tokio::test(flavor = "multi_thread")]
async fn invite_with_existing_expired_invite() {
    let (app, _, _, owner) = TestApp::init().with_token().await;
    let mut conn = app.db_conn();

    app.db_new_user("invited_user").await;
    let krate = CrateBuilder::new("crate_name", owner.as_model().user_id).expect_build(&mut conn);

    // Ensure no emails were sent up to this point
    assert_eq!(app.emails().await.len(), 0);

    // Invite the user the first time
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check one email was sent, this will be the ownership invite email
    assert_eq!(app.emails().await.len(), 1);

    // Simulate the previous invite expiring
    expire_invitation(&app, krate.id);

    // Then invite the user a second time, a new invite is created as the old one expired
    let response = owner.add_named_owner("crate_name", "invited_user").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user invited_user has been invited to be an owner of crate crate_name","ok":true}"#);

    // Check that the email for the second invite was sent
    assert_eq!(app.emails().await.len(), 2);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_crate() {
    let (app, _, user) = TestApp::full().with_user().await;
    app.db_new_user("bar").await;

    let response = user.add_named_owner("unknown", "bar").await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `unknown` does not exist"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_user() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("foo", cookie.as_model().id).expect_build(&mut conn);

    let response = cookie.add_named_owner("foo", "unknown").await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `unknown`"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn test_unknown_team() {
    let (app, _, cookie) = TestApp::full().with_user().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("foo", cookie.as_model().id).expect_build(&mut conn);

    let response = cookie
        .add_named_owner("foo", "github:unknown:unknown")
        .await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find the github team unknown/unknown. Make sure that you have the right permissions in GitHub. See https://doc.rust-lang.org/cargo/reference/publishing.html#github-permissions"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn max_invites_per_request() {
    let (app, _, _, owner) = TestApp::init().with_token().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("crate_name", owner.as_model().user_id).expect_build(&mut conn);

    let usernames = (0..11)
        .map(|i| format!("user_{i}"))
        .collect::<Vec<String>>();

    // Populate enough users in the database to submit 11 invites at once.
    for user in &usernames {
        app.db_new_user(user).await;
    }

    let response = owner.add_named_owners("crate_name", &usernames).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"too many invites for this request - maximum 10"}]}"#);
}

/// Assert that emails are only sent if the request succeeds.
#[tokio::test(flavor = "multi_thread")]
async fn no_invite_emails_for_txn_rollback() {
    let (app, _, _, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn();

    CrateBuilder::new("crate_name", token.as_model().user_id).expect_build(&mut conn);

    let mut usernames = (0..9).map(|i| format!("user_{i}")).collect::<Vec<String>>();

    // Populate enough users in the database to submit 9 good invites.
    for user in &usernames {
        app.db_new_user(user).await;
    }

    // Add an invalid username to the end of the invite list to cause the
    // request to fail.
    usernames.push("bananas".to_string());

    let response = token.add_named_owners("crate_name", &usernames).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"could not find user with login `bananas`"}]}"#);

    // No emails should have been sent.
    assert_eq!(app.emails().await.len(), 0);

    // Remove the bad username.
    let _ = usernames.pop();

    let response = token.add_named_owners("crate_name", &usernames).await;
    assert_eq!(response.status(), StatusCode::OK);

    // 9 emails to the good invitees should have been sent.
    assert_eq!(app.emails().await.len(), 9);
}
