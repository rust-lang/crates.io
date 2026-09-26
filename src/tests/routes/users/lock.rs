use claims::assert_none;
use crates_io::{controllers::user::lock::UserLockGetResponse, schema::users};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::assert_json_snapshot;

use crate::util::{RequestHelper, TestApp};

#[tokio::test(flavor = "multi_thread")]
async fn get_anon() {
    let (_, anon, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Anonymous users should not have access.
    let response = anon.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_regular_user() {
    let (_, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Normal users should not have access.
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_admin_unlocked() {
    let (app, _, user) = TestApp::init().with_user().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Now we should be able to see that the user is not currently locked.
    let response = admin.get::<UserLockGetResponse>(&url).await.good();
    assert_none!(response.lock);
}

#[tokio::test(flavor = "multi_thread")]
async fn get_admin_locked() -> anyhow::Result<()> {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    let url = format!("/api/v1/users/{}/lock", user.as_model().username);

    // Let's create an admin.
    let admin = app.db_new_admin_user("admin").await;

    // Let's lock the user.
    diesel::update(users::table)
        .set(users::account_lock_reason.eq(Some("test")))
        .filter(users::id.eq(user.as_model().id))
        .execute(&mut conn)
        .await?;

    let response = admin.get::<UserLockGetResponse>(&url).await.good();
    assert_json_snapshot!(response, @r#"
    {
      "lock": {
        "reason": "test",
        "until": null
      }
    }
    "#);

    Ok(())
}
