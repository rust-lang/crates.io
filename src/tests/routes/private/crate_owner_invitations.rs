//! Tests for the `GET /api/private/crate-owners-invitations` endpoint

use crate::builders::CrateBuilder;
use crate::util::{MockCookieUser, RequestHelper, TestApp};
use crates_io::views::{EncodableCrateOwnerInvitation, EncodablePublicUser};
use http::StatusCode;
use insta::assert_json_snapshot;
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct CrateOwnerInvitationsResponse {
    invitations: Vec<EncodableCrateOwnerInvitation>,
    users: Vec<EncodablePublicUser>,
    meta: CrateOwnerInvitationsMeta,
}
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
struct CrateOwnerInvitationsMeta {
    next_page: Option<String>,
}

async fn get_invitations(user: &MockCookieUser, query: &str) -> CrateOwnerInvitationsResponse {
    user.get_with_query::<CrateOwnerInvitationsResponse>(
        "/api/private/crate_owner_invitations",
        query,
    )
    .await
    .good()
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_list() {
    let (app, _, owner, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;

    let _crate1 = CrateBuilder::new("crate_1", owner.as_model().id)
        .expect_build(&mut conn)
        .await;
    let _crate2 = CrateBuilder::new("crate_2", owner.as_model().id)
        .expect_build(&mut conn)
        .await;

    let user1 = app.db_new_user("user_1").await;
    let user2 = app.db_new_user("user_2").await;
    token.add_named_owner("crate_1", "user_1").await.good();
    token.add_named_owner("crate_1", "user_2").await.good();
    token.add_named_owner("crate_2", "user_1").await.good();

    // user1 has invites for both crates
    let invitations = get_invitations(&user1, &format!("invitee_id={}", user1.as_model().id)).await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });

    // user2 is only invited to a single crate
    let invitations = get_invitations(&user2, &format!("invitee_id={}", user2.as_model().id)).await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });

    // owner has no invites
    let invitations = get_invitations(&owner, &format!("invitee_id={}", owner.as_model().id)).await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });

    // crate1 has two available invitations
    let invitations = get_invitations(&owner, "crate_name=crate_1").await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });

    // crate2 has one available invitation
    let invitations = get_invitations(&owner, "crate_name=crate_2").await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_does_not_include_expired_invites() {
    let (app, _, owner, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("invited_user").await;

    let crate1 = CrateBuilder::new("crate_1", owner.as_model().id)
        .expect_build(&mut conn)
        .await;
    let _crate2 = CrateBuilder::new("crate_2", owner.as_model().id)
        .expect_build(&mut conn)
        .await;

    token
        .add_named_owner("crate_1", "invited_user")
        .await
        .good();
    token
        .add_named_owner("crate_2", "invited_user")
        .await
        .good();

    // Simulate one of the invitations expiring
    crate::owners::expire_invitation(&app, crate1.id).await;

    // user1 has an invite just for crate 2
    let invitations = get_invitations(&user, &format!("invitee_id={}", user.as_model().id)).await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_paginated() {
    let (app, _, owner, token) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let user = app.db_new_user("invited_user").await;

    let _crate1 = CrateBuilder::new("crate_1", owner.as_model().id)
        .expect_build(&mut conn)
        .await;
    let _crate2 = CrateBuilder::new("crate_2", owner.as_model().id)
        .expect_build(&mut conn)
        .await;

    token
        .add_named_owner("crate_1", "invited_user")
        .await
        .good();
    token
        .add_named_owner("crate_2", "invited_user")
        .await
        .good();

    // Fetch the first page of results
    let invitations = get_invitations(
        &user,
        &format!("per_page=1&invitee_id={}", user.as_model().id),
    )
    .await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });

    // Fetch the second page of results
    let invitations = get_invitations(
        &user,
        invitations.meta.next_page.unwrap().trim_start_matches('?'),
    )
    .await;
    assert_json_snapshot!(invitations, {
        ".invitations[].created_at" => "[datetime]",
        ".invitations[].expires_at" => "[datetime]",
        ".users[].created_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_list_with_no_filter() {
    let (_, _, owner, _) = TestApp::init().with_token().await;

    let resp = owner
        .get::<()>("/api/private/crate_owner_invitations")
        .await;
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.json(),
        json!({
            "errors": [{
                "detail": "missing or invalid filter",
            }],
        })
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_list_other_users() {
    let (app, _, owner, _) = TestApp::init().with_token().await;
    let other_user = app.db_new_user("other").await;

    // Retrieving our own invitations work.
    let resp = owner
        .get_with_query::<()>(
            "/api/private/crate_owner_invitations",
            &format!("invitee_id={}", owner.as_model().id),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Retrieving other users' invitations doesn't work.
    let resp = owner
        .get_with_query::<()>(
            "/api/private/crate_owner_invitations",
            &format!("invitee_id={}", other_user.as_model().id),
        )
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_list_other_crates() {
    let (app, _, owner, _) = TestApp::init().with_token().await;
    let mut conn = app.db_conn().await;
    let other_user = app.db_new_user("other").await;

    CrateBuilder::new("crate_1", owner.as_model().id)
        .expect_build(&mut conn)
        .await;
    CrateBuilder::new("crate_2", other_user.as_model().id)
        .expect_build(&mut conn)
        .await;

    // Retrieving our own invitations work.
    let resp = owner
        .get_with_query::<()>("/api/private/crate_owner_invitations", "crate_name=crate_1")
        .await;
    assert_eq!(resp.status(), StatusCode::OK);

    // Retrieving other users' invitations doesn't work.
    let resp = owner
        .get_with_query::<()>("/api/private/crate_owner_invitations", "crate_name=crate_2")
        .await;
    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}
