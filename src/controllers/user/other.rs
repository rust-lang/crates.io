use crate::controllers::prelude::*;

use crate::models::{OwnerKind, User};
use crate::schema::{crate_owners, crates, users};
use crate::views::EncodablePublicUser;

/// Handles the `GET /users/:user_id` route.
pub fn show(req: &mut dyn Request) -> CargoResult<Response> {
    use self::users::dsl::{gh_deleted, gh_login, id, users};

    let name = &req.params()["user_id"].to_lowercase();
    let conn = req.db_conn()?;
    let mut query = users
        .filter(crate::lower(gh_login).eq(name))
        .order(id.desc())
        .into_boxed();
    if name.starts_with("deleted_") {
        if let Ok(deleted_id) = name["deleted_".len()..].parse::<i32>() {
            query = query.or_filter(id.eq(deleted_id).and(gh_deleted));
        }
    };

    let user = query.first::<User>(&conn)?;

    #[derive(Serialize)]
    struct R {
        user: EncodablePublicUser,
    }
    Ok(req.json(&R {
        user: user.encodable_public(),
    }))
}

/// Handles the `GET /users/:user_id/stats` route.
pub fn stats(req: &mut dyn Request) -> CargoResult<Response> {
    use diesel::dsl::sum;

    let user_id = &req.params()["user_id"].parse::<i32>().ok().unwrap();
    let conn = req.db_conn()?;

    let data = crate_owners::table
        .inner_join(crates::table)
        .filter(
            crate_owners::owner_id
                .eq(user_id)
                .and(crate_owners::owner_kind.eq(OwnerKind::User as i32)),
        )
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
