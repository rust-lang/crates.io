use crate::{util::RequestHelper, TestApp};
use chrono::{NaiveDateTime, Utc};
use conduit::StatusCode;
use diesel::prelude::*;

mod rate_limits {
    use super::*;

    const RATE_LIMITS_URL: &str = "/api/private/admin/rate-limits";

    #[test]
    fn anon_sending_rate_limit_changes_returns_unauthorized() {
        let (_app, anon) = TestApp::init().empty();
        anon.put(RATE_LIMITS_URL, &[]).assert_forbidden();
    }

    #[test]
    fn non_admin_sending_rate_limit_changes_returns_unauthorized() {
        let (app, _anon) = TestApp::init().empty();
        let user = app.db_new_user("foo");
        user.put(RATE_LIMITS_URL, &[]).assert_forbidden();
    }

    #[test]
    fn no_body_content_returns_400() {
        let (app, _anon) = TestApp::init().empty();
        let admin_user = app.db_new_user("carols10cents");
        let response = admin_user.put::<()>(RATE_LIMITS_URL, &[]);

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.into_json(),
            json!({ "errors": [{
                "detail": "invalid json request: EOF while parsing a value at line 1 column 0"
            }] })
        );
    }

    #[test]
    fn rate_limit_nan_returns_400() {
        let (app, _anon) = TestApp::init().empty();
        let admin_user = app.db_new_user("carols10cents");

        let body = json!({
            "email": "foo@example.com",
            "rate_limit": "-34g",
        });

        let response = admin_user.put::<()>(RATE_LIMITS_URL, body.to_string().as_bytes());

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.into_json(),
            json!({ "errors": [{
                "detail": "invalid json request: invalid type: string \"-34g\", expected i32 at \
                line 1 column 46"
            }] })
        );
    }

    #[test]
    fn email_address_lookup_failure_returns_not_found() {
        let (app, _anon) = TestApp::init().empty();
        let admin_user = app.db_new_user("carols10cents");

        let body = json!({
            "email": "foo@example.com",
            "rate_limit": 88,
        });

        let response = admin_user.put::<()>(RATE_LIMITS_URL, body.to_string().as_bytes());

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        assert_eq!(
            response.into_json(),
            json!({ "errors": [{
                "detail": "Not Found"
            }] })
        );
    }

    #[test]
    fn email_address_lookup_success_updates_rate_limit() {
        use cargo_registry::{
            publish_rate_limit::PublishRateLimit,
            schema::{publish_limit_buckets, publish_rate_overrides},
        };
        use std::time::Duration;

        let (app, _, user) = TestApp::init().with_user();
        let user_model = user.as_model();

        // Check the rate limit for a user, which inserts a record for the user into the
        // `publish_limit_buckets` table so that we can test it gets deleted by the rate limit
        // override.
        app.db(|conn| {
            let rate = PublishRateLimit {
                rate: Duration::from_secs(1),
                burst: 10,
            };
            rate.check_rate_limit(user_model.id, conn).unwrap();
        });

        let admin_user = app.db_new_user("carols10cents");

        let email = app.db(|conn| user_model.email(conn).unwrap());
        let new_rate_limit = 88;
        let body = json!({
            "email": email,
            "rate_limit": new_rate_limit,
        });
        let response = admin_user.put::<()>(RATE_LIMITS_URL, body.to_string().as_bytes());

        assert_eq!(response.status(), StatusCode::OK);

        let (rate_limit, expires_at): (i32, Option<NaiveDateTime>) = app
            .db(|conn| {
                publish_rate_overrides::table
                    .select((
                        publish_rate_overrides::burst,
                        publish_rate_overrides::expires_at,
                    ))
                    .filter(publish_rate_overrides::user_id.eq(user_model.id))
                    .first(conn)
            })
            .unwrap();
        assert_eq!(rate_limit, new_rate_limit);
        assert_eq!(
            expires_at.unwrap().date(),
            (Utc::now() + chrono::Duration::days(30)).naive_utc().date()
        );
        app.db(|conn| {
            assert!(!diesel::select(diesel::dsl::exists(
                publish_limit_buckets::table
                    .filter(publish_limit_buckets::user_id.eq(user_model.id))
            ))
            .get_result::<bool>(conn)
            .unwrap());
        });
    }
}
