use crate::controllers::frontend_prelude::*;

use crate::models::{CrateOwner, OwnerKind, User};
use crate::schema::{crate_owners, crates, users};
use crate::util::errors::ChainError;
use crate::views::EncodablePublicUser;

/// Handles the `GET /users/:user_id` route.
pub fn show(req: &mut dyn Request) -> AppResult<Response> {
    use self::users::dsl::{gh_login, id, users};

    let name = &req.params()["user_id"].to_lowercase();
    let conn = req.db_conn()?;
    let user = users
        .filter(crate::lower(gh_login).eq(name))
        .order(id.desc())
        .first::<User>(&*conn)?;

    #[derive(Serialize)]
    struct R {
        user: EncodablePublicUser,
    }
    Ok(req.json(&R {
        user: user.encodable_public(),
    }))
}

/// Handles the `GET /users/:user_id/stats` route.
pub fn stats(req: &mut dyn Request) -> AppResult<Response> {
    use diesel::dsl::sum;

    let user_id = &req.params()["user_id"]
        .parse::<i32>()
        .chain_error(|| bad_request("invalid user_id"))?;
    let conn = req.db_conn()?;

    let data = CrateOwner::by_owner_kind(OwnerKind::User)
        .inner_join(crates::table)
        .filter(crate_owners::owner_id.eq(user_id))
        .select(sum(crates::downloads))
        .first::<Option<i64>>(&*conn)?
        .unwrap_or(0);

    #[derive(Serialize)]
    struct R {
        total_downloads: i64,
    }
    Ok(req.json(&R {
        total_downloads: data,
    }))
}
