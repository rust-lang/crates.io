use crate::builders::{CrateBuilder, UserBuilder};
use crate::util::{MockCookieUser, RequestHelper, TestApp};
use crates_io::schema::crate_owner_invitations;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use insta::{assert_json_snapshot, assert_snapshot};

const ACCEPTED: &str = r#"{"crate_owner_invitation":{"state":"accepted"}}"#;
const DECLINED: &str = r#"{"crate_owner_invitation":{"state":"declined"}}"#;

struct Fixture {
    app: TestApp,
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
    let (app, _, owner, token) = TestApp::full().with_token().await;
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
    let Fixture { owner, url, .. } = invite().await?;

    let response = owner.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"404 Not Found");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Not Found"}]}"#);

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
async fn tokens_are_refused() -> anyhow::Result<()> {
    let Fixture { invitee, url, .. } = invite().await?;
    let token = invitee.db_new_token("t").await;

    let response = token.patch::<()>(&url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn anonymous() {
    let (_, anon) = TestApp::init().empty().await;

    let url = "/api/v1/crate_owner_invitations/42";
    let response = anon.patch::<()>(url, ACCEPTED).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}
