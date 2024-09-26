use crate::schema::crate_owners;
use crate::tests::builders::CrateBuilder;
use crate::tests::new_user;
use crate::tests::util::{RequestHelper, TestApp};
use diesel::prelude::*;
use http::StatusCode;

#[derive(Serialize)]
struct EmailNotificationsUpdate {
    id: i32,
    email_notifications: bool,
}

impl crate::tests::util::MockCookieUser {
    async fn update_email_notifications(&self, updates: Vec<EmailNotificationsUpdate>) {
        let response = self
            .put::<()>("/api/v1/me/email_notifications", json!(updates).to_string())
            .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.json(), json!({ "ok": true }));
    }
}

/// A user should be able to update the email notifications for crates they own. Only the crates that
/// were sent in the request should be updated to the corresponding `email_notifications` value.
#[tokio::test(flavor = "multi_thread")]
async fn test_update_email_notifications() {
    let (app, _, user) = TestApp::init().with_user();

    let my_crates = app.db(|conn| {
        vec![
            CrateBuilder::new("test_package", user.as_model().id).expect_build(conn),
            CrateBuilder::new("another_package", user.as_model().id).expect_build(conn),
        ]
    });

    let a_id = my_crates.first().unwrap().id;
    let b_id = my_crates.get(1).unwrap().id;

    // Update crate_a: email_notifications = false
    // crate_a should be false, crate_b should be true
    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: a_id,
        email_notifications: false,
    }])
    .await;
    let json = user.show_me().await;

    assert!(
        !json
            .owned_crates
            .iter()
            .find(|c| c.id == a_id)
            .unwrap()
            .email_notifications
    );
    assert!(
        json.owned_crates
            .iter()
            .find(|c| c.id == b_id)
            .unwrap()
            .email_notifications
    );

    // Update crate_b: email_notifications = false
    // Both should be false now
    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: b_id,
        email_notifications: false,
    }])
    .await;
    let json = user.show_me().await;

    assert!(
        !json
            .owned_crates
            .iter()
            .find(|c| c.id == a_id)
            .unwrap()
            .email_notifications
    );
    assert!(
        !json
            .owned_crates
            .iter()
            .find(|c| c.id == b_id)
            .unwrap()
            .email_notifications
    );

    // Update crate_a and crate_b: email_notifications = true
    // Both should be true
    user.update_email_notifications(vec![
        EmailNotificationsUpdate {
            id: a_id,
            email_notifications: true,
        },
        EmailNotificationsUpdate {
            id: b_id,
            email_notifications: true,
        },
    ])
    .await;
    let json = user.show_me().await;

    json.owned_crates.iter().for_each(|c| {
        assert!(c.email_notifications);
    })
}

/// A user should not be able to update the `email_notifications` value for a crate that is not
/// owned by them.
#[tokio::test(flavor = "multi_thread")]
async fn test_update_email_notifications_not_owned() {
    let (app, _, user) = TestApp::init().with_user();

    let not_my_crate = app.db(|conn| {
        let u = new_user("arbitrary_username")
            .create_or_update(None, &app.as_inner().emails, conn)
            .unwrap();
        CrateBuilder::new("test_package", u.id).expect_build(conn)
    });

    user.update_email_notifications(vec![EmailNotificationsUpdate {
        id: not_my_crate.id,
        email_notifications: false,
    }])
    .await;

    let email_notifications: bool = app
        .db(|conn| {
            crate_owners::table
                .select(crate_owners::email_notifications)
                .filter(crate_owners::crate_id.eq(not_my_crate.id))
                .first(conn)
        })
        .unwrap();

    // There should be no change to the `email_notifications` value for a crate not belonging to me
    assert!(email_notifications);
}
