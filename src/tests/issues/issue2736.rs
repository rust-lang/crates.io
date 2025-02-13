use crate::models::CrateOwner;
use crate::tests::builders::CrateBuilder;
use crate::tests::util::{RequestHelper, TestApp};
use crates_io_database::schema::users;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use http::StatusCode;
use insta::assert_snapshot;

/// See <https://github.com/rust-lang/crates.io/issues/2736>.
#[tokio::test(flavor = "multi_thread")]
async fn test_issue_2736() -> anyhow::Result<()> {
    let (app, _) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    // - A user had a GitHub account named, let's say, `foo`
    let foo1 = app.db_new_user("foo").await;

    // - Another user `someone_else` added them as an owner of a crate
    let someone_else = app.db_new_user("someone_else").await;

    let krate = CrateBuilder::new("crate1", someone_else.as_model().id)
        .expect_build(&mut conn)
        .await;

    CrateOwner::builder()
        .crate_id(krate.id)
        .user_id(foo1.as_model().id)
        .created_by(someone_else.as_model().id)
        .build()
        .insert(&mut conn)
        .await?;

    // - `foo` deleted their GitHub account (but crates.io has no real knowledge of this)
    // - `foo` recreated their GitHub account with the same username (because it was still available), but in this situation GitHub assigns them a new ID
    // - When `foo` now logs in to crates.io, it's a different account than their old `foo` crates.io account because of the new GitHub ID (and if it wasn't, this would be a security problem)
    let foo2 = app.db_new_user("foo").await;

    let github_ids = users::table
        .filter(users::gh_login.eq("foo"))
        .select(users::gh_id)
        .load::<i32>(&mut conn)
        .await?;

    assert_eq!(github_ids.len(), 2);
    assert_ne!(github_ids[0], github_ids[1]);

    // - The new `foo` account is NOT an owner of the crate
    let owners = krate.owners(&mut conn).await?;
    assert_eq!(owners.len(), 2);
    assert_none!(owners.iter().find(|o| o.id() == foo2.as_model().id));

    // Removing an owner, whether it's valid/current or not, should always work (if performed by another valid owner, etc)
    let response = someone_else.remove_named_owner("crate1", "foo").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"owners successfully removed","ok":true}"#);

    let owners = krate.owners(&mut conn).await?;
    assert_eq!(owners.len(), 1);
    assert_eq!(owners[0].id(), someone_else.as_model().id);

    // Once that removal works, it should be possible to add the new account as an owner
    let response = someone_else.add_named_owner("crate1", "foo").await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_snapshot!(response.text(), @r#"{"msg":"user foo has been invited to be an owner of crate crate1","ok":true}"#);

    Ok(())
}
