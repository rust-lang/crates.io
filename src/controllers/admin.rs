use super::frontend_prelude::*;
use crate::{
    models::{AdminUser, User},
    schema::{publish_limit_buckets, publish_rate_overrides},
};
use diesel::dsl::*;

#[derive(Deserialize)]
struct RateLimitIncrease {
    email: String,
    rate_limit: i32,
}

/// Increases the rate limit for the user with the specified verified email address.
pub fn publish_rate_override(req: &mut dyn RequestExt) -> EndpointResult {
    let admin = req.authenticate()?.forbid_api_token_auth()?.admin_user()?;
    increase_rate_limit(admin, req)
}

/// Increasing the rate limit requires that you are an admin user, but no information from the
/// admin user is currently needed. Someday having an audit log of which admin user took the action
/// would be nice.
fn increase_rate_limit(_admin: AdminUser, req: &mut dyn RequestExt) -> EndpointResult {
    let mut body = String::new();
    req.body().read_to_string(&mut body)?;

    let rate_limit_increase: RateLimitIncrease = serde_json::from_str(&body)
        .map_err(|e| bad_request(&format!("invalid json request: {e}")))?;

    let conn = req.db_write()?;
    let user = User::find_by_verified_email(&conn, &rate_limit_increase.email)?;

    conn.transaction(|| {
        diesel::insert_into(publish_rate_overrides::table)
            .values((
                publish_rate_overrides::user_id.eq(user.id),
                publish_rate_overrides::burst.eq(rate_limit_increase.rate_limit),
                publish_rate_overrides::expires_at.eq((now + 30.days()).nullable()),
            ))
            .execute(&*conn)?;

        diesel::delete(publish_limit_buckets::table)
            .filter(publish_limit_buckets::user_id.eq(user.id))
            .execute(&*conn)
    })?;

    ok_true()
}
