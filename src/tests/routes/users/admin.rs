use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

use crate::tests::util::{RequestHelper, TestApp};

mod get {
    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn get() {
        let (app, anon, user) = TestApp::init().with_user().await;
        let admin = app.db_new_admin_user("admin").await;

        // Anonymous users should be forbidden.
        let response = anon.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-found", response.text());

        let response = anon.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("anonymous-not-found", response.text());

        // Regular users should also be forbidden, even if they're requesting
        // themself.
        let response = user.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-found", response.text());

        let response = user.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        assert_snapshot!("non-admin-not-found", response.text());

        // Admin users are allowed, but still can't manifest users who don't
        // exist.
        let response = admin.get::<()>("/api/v1/users/bar/admin").await;
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_snapshot!("admin-not-found", response.text());

        // Admin users are allowed, and should get an admin's eye view of the
        // requested user.
        let response = admin.get::<()>("/api/v1/users/foo/admin").await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_json_snapshot!("admin-found", response.json());
    }
}
