use crate::builders::{CrateBuilder, UserBuilder};
use crate::util::{MockAnonymousUser, MockCookieUser, RequestHelper, TestApp};
use crates_io::models::CrateOwner;
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::schema::crate_owner_invitations;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};
use serde_json::json;

const ACCEPTED: &str = r#"{"crate_owner_invitation":{"state":"accepted"}}"#;
const DECLINED: &str = r#"{"crate_owner_invitation":{"state":"declined"}}"#;
const REVOKED: &str = r#"{"crate_owner_invitation":{"state":"revoked"}}"#;

struct Fixture {
    app: TestApp,
    anon: MockAnonymousUser,
    owner: MockCookieUser,
    invitee: MockCookieUser,
    crate_id: i32,
    url: String,
}

/// Creates a `foo` crate and invites a new `invitee` user to become an owner.
async fn invite() -> anyhow::Result<Fixture> {
    invite_user(UserBuilder::new("invitee")).await
}

/// Creates a `foo` crate and invites the `invitee` user to become an owner.
async fn invite_user(invitee: UserBuilder<'_>) -> anyhow::Result<Fixture> {
    let (app, anon, owner, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;
    let invitee = app.db_new_user_from_builder(invitee).await;

    let owner_id = owner.as_model().id;
    let krate = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    token.add_named_owner("foo", "invitee").await.good();

    let query = crate_owner_invitations::table.select(crate_owner_invitations::id);
    let id: i64 = query.first(&mut conn).await?;
    let url = format!("/api/v1/crate_owner_invitations/{id}");

    Ok(Fixture {
        app,
        anon,
        owner,
        invitee,
        crate_id: krate.id,
        url,
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn accept() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;

    let response = invitee.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });
    assert_eq!(invitee.show_crate_owners("foo").await.users.len(), 2);

    let response = invitee.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"404 Not Found");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn decline() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;

    let response = invitee.patch::<()>(&url, DECLINED).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });
    assert_eq!(invitee.show_crate_owners("foo").await.users.len(), 1);

    let response = invitee.patch::<()>(&url, DECLINED).await;
    assert_snapshot!(response.status(), @"404 Not Found");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn other_user() -> anyhow::Result<()> {
    let Fixture { app, url, .. } = invite().await?;
    let user = app.db_new_user("other").await;

    let response = user.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only the invitee or a crate owner can update this invitation"}]}"#);

    let response = user.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only the invitee or a crate owner can update this invitation"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unknown_id() {
    let (_, _, user) = TestApp::init().with_user().await;

    let url = "/api/v1/crate_owner_invitations/42";
    let response = user.patch::<()>(url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn expired() -> anyhow::Result<()> {
    let fixture = invite().await?;
    crate::owners::expire_invitation(&fixture.app, fixture.crate_id).await;

    let response = fixture.invitee.patch::<()>(&fixture.url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"410 Gone");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"The invitation to become an owner of the foo crate expired. Please reach out to an owner of the crate to request a new invitation."}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn unverified_email() -> anyhow::Result<()> {
    let builder = UserBuilder::new("invitee").with_email_verified(false);
    let Fixture { invitee, url, .. } = invite_user(builder).await?;

    let response = invitee.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"You need to verify your email address before you can accept the invitation to become an owner of the foo crate."}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_state() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;

    let body = r#"{"crate_owner_invitation":{"state":"pending"}}"#;
    let response = invitee.patch::<()>(&url, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"`state` must be `accepted` or `declined`"}]}"#);

    let body = r#"{"crate_owner_invitation":{"state":"revoked"}}"#;
    let response = invitee.patch::<()>(&url, body).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"`state` must be `accepted` or `declined`"}]}"#);

    let body = r#"{"crate_owner_invitation":{"state":"foo"}}"#;
    let response = invitee.patch::<()>(&url, body).await;
    assert_snapshot!(response.status(), @"422 Unprocessable Entity");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Failed to deserialize the JSON body into the target type: crate_owner_invitation.state: unknown variant `foo`, expected one of `pending`, `accepted`, `declined`, `revoked` at line 1 column 40"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn token_with_scope() -> anyhow::Result<()> {
    let Fixture { invitee, .. } = invite().await?;
    let foo = Some(vec![CrateScope::try_from("foo*").unwrap()]);
    let scopes = Some(vec![EndpointScope::CrateOwnerInvitations]);
    let token = invitee.db_new_scoped_token("ci", foo, scopes, None).await;

    let query = format!("invitee_id={}", invitee.as_model().id);
    let url = "/api/v1/crate_owner_invitations";
    let response = token.get_with_query::<()>(url, &query).await;
    let id = &response.json()["crate_owner_invitations"][0]["id"];

    let url = format!("/api/v1/crate_owner_invitations/{id}");
    let response = token.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });
    assert_eq!(invitee.show_crate_owners("foo").await.users.len(), 2);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn token_without_scope() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;
    let scopes = Some(vec![EndpointScope::ChangeOwners]);
    let token = invitee.db_new_scoped_token("ci", None, scopes, None).await;

    let response = token.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn token_with_other_crate_scope() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;
    let bar = Some(vec![CrateScope::try_from("bar").unwrap()]);
    let scopes = Some(vec![EndpointScope::CrateOwnerInvitations]);
    let token = invitee.db_new_scoped_token("ci", bar, scopes, None).await;

    let response = token.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn legacy_token() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;
    let token = invitee.db_new_token("t").await;

    let response = token.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This endpoint requires a scoped API token. Create a new token with the `crate-owner-invitations` scope at https://crates.io/settings/tokens/new"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn anonymous() -> anyhow::Result<()> {
    let Fixture { anon, url, .. } = invite().await?;

    let response = anon.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke() -> anyhow::Result<()> {
    let fixture = invite().await?;
    let Fixture { owner, invitee, .. } = &fixture;
    let emails = fixture.app.emails().await.len();

    let url = &fixture.url;
    let response = owner.patch::<()>(url, REVOKED).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });
    assert_eq!(fixture.app.emails().await.len(), emails);

    let list_url = "/api/v1/crate_owner_invitations";
    let response = owner.get_with_query::<()>(list_url, "crate_name=foo").await;
    assert_snapshot!(response.text(), @r#"{"crate_owner_invitations":[],"users":[],"meta":{"next_page":null}}"#);

    let query = format!("invitee_id={}", invitee.as_model().id);
    let response = invitee.get_with_query::<()>(list_url, &query).await;
    assert_snapshot!(response.text(), @r#"{"crate_owner_invitations":[],"users":[],"meta":{"next_page":null}}"#);

    let response = invitee.patch::<()>(url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"404 Not Found");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_by_other_owner() -> anyhow::Result<()> {
    let fixture = invite().await?;
    let conn = fixture.app.db_conn().await;
    let other = fixture.app.db_new_user("other").await;

    CrateOwner::builder()
        .crate_id(fixture.crate_id)
        .user_id(other.as_model().id)
        .created_by(fixture.owner.as_model().id)
        .build()
        .insert(&conn)
        .await?;

    let response = other.patch::<()>(&fixture.url, REVOKED).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_by_former_owner() -> anyhow::Result<()> {
    let fixture = invite().await?;
    let conn = fixture.app.db_conn().await;
    let owner = &fixture.owner;
    let other = fixture.app.db_new_user("other").await;

    CrateOwner::builder()
        .crate_id(fixture.crate_id)
        .user_id(other.as_model().id)
        .created_by(owner.as_model().id)
        .build()
        .insert(&conn)
        .await?;

    owner.remove_named_owner("foo", "other").await.good();

    let response = other.patch::<()>(&fixture.url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only the invitee or a crate owner can update this invitation"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_by_team_owner() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;
    let owner = app.db_new_user("user-org-owner").await;
    let member = app.db_new_user("user-one-team").await;
    app.db_new_user("invitee").await;

    let owner_id = owner.as_model().id;
    CrateBuilder::new("foo", owner_id).build(&mut conn).await?;

    let body = json!({ "owners": ["github:test-org:all", "invitee"] }).to_string();
    let response = owner.put::<()>("/api/v1/crates/foo/owners", body).await;
    assert_snapshot!(response.status(), @"200 OK");

    let query = crate_owner_invitations::table.select(crate_owner_invitations::id);
    let id: i64 = query.first(&mut conn).await?;
    let url = format!("/api/v1/crate_owner_invitations/{id}");

    let response = member.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only the invitee or a crate owner can update this invitation"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn owner_invalid_state() -> anyhow::Result<()> {
    let Fixture { owner, url, .. } = invite().await?;

    let response = owner.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"`state` must be `revoked`"}]}"#);

    let response = owner.patch::<()>(&url, DECLINED).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"`state` must be `revoked`"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_with_token() -> anyhow::Result<()> {
    let Fixture { owner, url, .. } = invite().await?;
    let foo = Some(vec![CrateScope::try_from("foo*").unwrap()]);
    let scopes = Some(vec![EndpointScope::ChangeOwners]);
    let token = owner.db_new_scoped_token("ci", foo, scopes, None).await;

    let response = token.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"200 OK");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_with_token_without_scope() -> anyhow::Result<()> {
    let Fixture { owner, url, .. } = invite().await?;
    let scopes = Some(vec![EndpointScope::CrateOwnerInvitations]);
    let token = owner.db_new_scoped_token("ci", None, scopes, None).await;

    let response = token.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_with_token_with_other_crate_scope() -> anyhow::Result<()> {
    let Fixture { owner, url, .. } = invite().await?;
    let bar = Some(vec![CrateScope::try_from("bar").unwrap()]);
    let scopes = Some(vec![EndpointScope::ChangeOwners]);
    let token = owner.db_new_scoped_token("ci", bar, scopes, None).await;

    let response = token.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this token does not have the required permissions to perform this action"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_with_legacy_token() -> anyhow::Result<()> {
    let Fixture { owner, url, .. } = invite().await?;
    let token = owner.db_new_token("t").await;

    let response = token.patch::<()>(&url, REVOKED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"This endpoint requires a scoped API token. Create a new token with the `change-owners` scope at https://crates.io/settings/tokens/new"}]}"#);

    Ok(())
}
