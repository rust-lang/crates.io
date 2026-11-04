use crate::builders::CrateBuilder;
use crate::util::{RequestHelper, TestApp};
use insta::{assert_json_snapshot, assert_snapshot};

const URL: &str = "/api/v1/crate_owner_invitations";

#[tokio::test(flavor = "multi_thread")]
async fn list_by_invitee_and_crate() -> anyhow::Result<()> {
    let (app, _, owner, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    let owner_id = owner.as_model().id;
    CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    CrateBuilder::new("bar", owner_id).build(&mut conn).await?;

    let user1 = app.db_new_user("user_1").await;
    let user2 = app.db_new_user("user_2").await;
    token.add_named_owner("foo", "user_1").await.good();
    token.add_named_owner("foo", "user_2").await.good();
    token.add_named_owner("bar", "user_1").await.good();

    let query = format!("invitee_id={}", user1.as_model().id);
    let response = user1.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });

    let query = format!("invitee_id={}", user2.as_model().id);
    let response = user2.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });

    let query = format!("invitee_id={}", owner.as_model().id);
    let response = owner.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json());

    let response = owner.get_with_query::<()>(URL, "crate_name=foo").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });

    let response = owner.get_with_query::<()>(URL, "crate_name=bar").await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn expired_invitations_are_excluded() -> anyhow::Result<()> {
    let (app, _, owner, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;
    let invitee = app.db_new_user("invitee").await;

    let owner_id = owner.as_model().id;
    let foo = CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    CrateBuilder::new("bar", owner_id).build(&mut conn).await?;

    token.add_named_owner("foo", "invitee").await.good();
    token.add_named_owner("bar", "invitee").await.good();
    crate::owners::expire_invitation(&app, foo.id).await;

    let query = format!("invitee_id={}", invitee.as_model().id);
    let response = invitee.get_with_query::<()>(URL, &query).await;
    let invitations = &response.json()["crate_owner_invitations"];
    assert_eq!(invitations.as_array().unwrap().len(), 1);
    assert_eq!(invitations[0]["crate_name"], "bar");

    let response = owner.get_with_query::<()>(URL, "crate_name=foo").await;
    assert_snapshot!(response.text(), @r#"{"crate_owner_invitations":[],"users":[],"meta":{"next_page":null}}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn paginated() -> anyhow::Result<()> {
    let (app, _, owner, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;
    let invitee = app.db_new_user("invitee").await;

    let owner_id = owner.as_model().id;
    CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    CrateBuilder::new("bar", owner_id).build(&mut conn).await?;

    token.add_named_owner("foo", "invitee").await.good();
    token.add_named_owner("bar", "invitee").await.good();

    let query = format!("per_page=1&invitee_id={}", invitee.as_model().id);
    let response = invitee.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"200 OK");
    let first_page = response.json();
    assert_json_snapshot!(first_page, { ".**.created_at, .**.expires_at" => "[datetime]" });

    let next_page = first_page["meta"]["next_page"].as_str().unwrap();
    let response = invitee.get_with_query::<()>(URL, &next_page[1..]).await;
    assert_snapshot!(response.status(), @"200 OK");
    assert_json_snapshot!(response.json(), { ".**.created_at, .**.expires_at" => "[datetime]" });

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn missing_filter() {
    let (_, _, owner) = TestApp::init().with_user().await;

    let response = owner.get::<()>(URL).await;
    assert_snapshot!(response.status(), @"400 Bad Request");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"missing or invalid filter"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn anonymous() {
    let (_, anon) = TestApp::init().empty().await;

    let response = anon.get_with_query::<()>(URL, "invitee_id=1").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action requires authentication"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn other_invitee() {
    let (app, _, owner) = TestApp::init().with_user().await;
    let other_user = app.db_new_user("other").await;

    let query = format!("invitee_id={}", owner.as_model().id);
    let response = owner.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"200 OK");

    let query = format!("invitee_id={}", other_user.as_model().id);
    let response = owner.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only the invitee can query their pending invitations"}]}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn crate_not_owned() -> anyhow::Result<()> {
    let (app, _, owner) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let owner_id = owner.as_model().id;
    let other_id = app.db_new_user("other").await.as_model().id;

    CrateBuilder::new("foo", owner_id).build(&mut conn).await?;
    CrateBuilder::new("bar", other_id).build(&mut conn).await?;

    let response = owner.get_with_query::<()>(URL, "crate_name=foo").await;
    assert_snapshot!(response.status(), @"200 OK");

    let response = owner.get_with_query::<()>(URL, "crate_name=bar").await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"only crate owners can query pending invitations for their crate"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn tokens_are_refused() {
    let (_, _, owner, token) = TestApp::init().with_token().await;

    let query = format!("invitee_id={}", owner.as_model().id);
    let response = token.get_with_query::<()>(URL, &query).await;
    assert_snapshot!(response.status(), @"403 Forbidden");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}"#);
}
