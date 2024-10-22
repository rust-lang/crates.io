//! Tests for the `GET /api/private/crate-owners-invitations` endpoint

use crate::tests::builders::CrateBuilder;
use crate::tests::util::{MockCookieUser, RequestHelper, TestApp};
use crate::views::{EncodableCrateOwnerInvitation, EncodablePublicUser};
use http::StatusCode;

#[derive(Deserialize, Debug, PartialEq, Eq)]
struct CrateOwnerInvitationsResponse {
    invitations: Vec<EncodableCrateOwnerInvitation>,
    users: Vec<EncodablePublicUser>,
    meta: CrateOwnerInvitationsMeta,
}
#[derive(Deserialize, Debug, PartialEq, Eq)]
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
    let (app, _, owner, token) = TestApp::init().with_token();
    let mut conn = app.db_conn();

    let crate1 = CrateBuilder::new("crate_1", owner.as_model().id).expect_build(&mut conn);
    let crate2 = CrateBuilder::new("crate_2", owner.as_model().id).expect_build(&mut conn);

    let user1 = app.db_new_user("user_1");
    let user2 = app.db_new_user("user_2");
    token.add_named_owner("crate_1", "user_1").await.good();
    token.add_named_owner("crate_1", "user_2").await.good();
    token.add_named_owner("crate_2", "user_1").await.good();

    // user1 has invites for both crates
    let invitations = get_invitations(&user1, &format!("invitee_id={}", user1.as_model().id)).await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[0].created_at,
                    expires_at: invitations.invitations[0].expires_at,
                },
                EncodableCrateOwnerInvitation {
                    crate_id: crate2.id,
                    crate_name: crate2.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[1].created_at,
                    expires_at: invitations.invitations[1].expires_at,
                },
            ],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into()
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // user2 is only invited to a single crate
    let invitations = get_invitations(&user2, &format!("invitee_id={}", user2.as_model().id)).await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate1.id,
                crate_name: crate1.name.clone(),
                invitee_id: user2.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user2.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // owner has no invites
    let invitations = get_invitations(&owner, &format!("invitee_id={}", owner.as_model().id)).await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![],
            users: vec![],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // crate1 has two available invitations
    let invitations = get_invitations(&owner, "crate_name=crate_1").await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name.clone(),
                    invitee_id: user1.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[0].created_at,
                    expires_at: invitations.invitations[0].expires_at,
                },
                EncodableCrateOwnerInvitation {
                    crate_id: crate1.id,
                    crate_name: crate1.name,
                    invitee_id: user2.as_model().id,
                    inviter_id: owner.as_model().id,
                    // The timestamps depend on when the test is run.
                    created_at: invitations.invitations[1].created_at,
                    expires_at: invitations.invitations[1].expires_at,
                },
            ],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into(),
                user2.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );

    // crate2 has one available invitation
    let invitations = get_invitations(&owner, "crate_name=crate_2").await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user1.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user1.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_does_not_include_expired_invites() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let mut conn = app.db_conn();
    let user = app.db_new_user("invited_user");

    let crate1 = CrateBuilder::new("crate_1", owner.as_model().id).expect_build(&mut conn);
    let crate2 = CrateBuilder::new("crate_2", owner.as_model().id).expect_build(&mut conn);

    token
        .add_named_owner("crate_1", "invited_user")
        .await
        .good();
    token
        .add_named_owner("crate_2", "invited_user")
        .await
        .good();

    // Simulate one of the invitations expiring
    crate::tests::owners::expire_invitation(&app, crate1.id);

    // user1 has an invite just for crate 2
    let invitations = get_invitations(&user, &format!("invitee_id={}", user.as_model().id)).await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn invitations_list_paginated() {
    let (app, _, owner, token) = TestApp::init().with_token();
    let mut conn = app.db_conn();
    let user = app.db_new_user("invited_user");

    let crate1 = CrateBuilder::new("crate_1", owner.as_model().id).expect_build(&mut conn);
    let crate2 = CrateBuilder::new("crate_2", owner.as_model().id).expect_build(&mut conn);

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
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate1.id,
                crate_name: crate1.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta {
                // This unwraps and then wraps again in Some() to ensure it's not None
                next_page: Some(invitations.meta.next_page.clone().unwrap()),
            },
        }
    );

    // Fetch the second page of results
    let invitations = get_invitations(
        &user,
        invitations.meta.next_page.unwrap().trim_start_matches('?'),
    )
    .await;
    assert_eq!(
        invitations,
        CrateOwnerInvitationsResponse {
            invitations: vec![EncodableCrateOwnerInvitation {
                crate_id: crate2.id,
                crate_name: crate2.name,
                invitee_id: user.as_model().id,
                inviter_id: owner.as_model().id,
                // The timestamps depend on when the test is run.
                created_at: invitations.invitations[0].created_at,
                expires_at: invitations.invitations[0].expires_at,
            }],
            users: vec![
                owner.as_model().clone().into(),
                user.as_model().clone().into(),
            ],
            meta: CrateOwnerInvitationsMeta { next_page: None },
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn invitation_list_with_no_filter() {
    let (_, _, owner, _) = TestApp::init().with_token();

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
    let (app, _, owner, _) = TestApp::init().with_token();
    let other_user = app.db_new_user("other");

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
    let (app, _, owner, _) = TestApp::init().with_token();
    let mut conn = app.db_conn();
    let other_user = app.db_new_user("other");

    CrateBuilder::new("crate_1", owner.as_model().id).expect_build(&mut conn);
    CrateBuilder::new("crate_2", other_user.as_model().id).expect_build(&mut conn);

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
