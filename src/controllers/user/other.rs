use crate::controllers::frontend_prelude::*;

use crate::models::{CrateOwner, OwnerKind, User};
use crate::schema::{crate_owners, crates, users};
use crate::sql::lower;
use crate::views::EncodablePublicUser;

/// Handles the `GET /users/:user_id` route.
pub fn show(req: &mut dyn RequestExt) -> EndpointResult {
    use self::users::dsl::{gh_login, id, users};

    let name = lower(&req.params()["user_id"]);
    let conn = req.db_conn()?;
    let user: User = users
        .filter(lower(gh_login).eq(name))
        .order(id.desc())
        .first(&*conn)?;

    Ok(req.json(&json!({ "user": EncodablePublicUser::from(user) })))
}

/// Handles the `GET /users/:user_id/stats` route.
pub fn stats(req: &mut dyn RequestExt) -> EndpointResult {
    use diesel::dsl::sum;

    let user_id = &req.params()["user_id"]
        .parse::<i32>()
        .map_err(|err| err.chain(bad_request("invalid user_id")))?;
    let conn = req.db_conn()?;

    let data: i64 = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .filter(crate_owners::owner_id.eq(user_id))
        .select(sum(crates::downloads))
        .first::<Option<i64>>(&*conn)?
        .unwrap_or(0);

    Ok(req.json(&json!({ "total_downloads": data })))
}
