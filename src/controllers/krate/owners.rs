//! All routes related to managing owners of a crate

use crate::auth::AuthCheck;
use crate::controllers::prelude::*;
use crate::models::token::EndpointScope;
use crate::models::{Crate, Owner, Rights, Team, User};
use crate::views::EncodableOwner;

/// Handles the `GET /crates/:crate_id/owners` route.
pub async fn owners(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let crate_name = req.param("crate_id").unwrap();
        let conn = req.app().db_read()?;
        let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
        let owners = krate
            .owners(&conn)?
            .into_iter()
            .map(Owner::into)
            .collect::<Vec<EncodableOwner>>();

        Ok(Json(json!({ "users": owners })))
    })
    .await
}

/// Handles the `GET /crates/:crate_id/owner_team` route.
pub async fn owner_team(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let crate_name = req.param("crate_id").unwrap();
        let conn = req.app().db_read()?;
        let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
        let owners = Team::owning(&krate, &conn)?
            .into_iter()
            .map(Owner::into)
            .collect::<Vec<EncodableOwner>>();

        Ok(Json(json!({ "teams": owners })))
    })
    .await
}

/// Handles the `GET /crates/:crate_id/owner_user` route.
pub async fn owner_user(req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || {
        let crate_name = req.param("crate_id").unwrap();
        let conn = req.app().db_read()?;
        let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
        let owners = User::owning(&krate, &conn)?
            .into_iter()
            .map(Owner::into)
            .collect::<Vec<EncodableOwner>>();

        Ok(Json(json!({ "users": owners })))
    })
    .await
}

/// Handles the `PUT /crates/:crate_id/owners` route.
pub async fn add_owners(mut req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || modify_owners(&mut req, true)).await
}

/// Handles the `DELETE /crates/:crate_id/owners` route.
pub async fn remove_owners(mut req: ConduitRequest) -> AppResult<Json<Value>> {
    conduit_compat(move || modify_owners(&mut req, false)).await
}

/// Parse the JSON request body of requests to modify the owners of a crate.
///
/// The format is:
///
/// ```json
/// {"owners": ["username", "github:org:team", ...]}
/// ```
fn parse_owners_request(req: &mut ConduitRequest) -> AppResult<Vec<String>> {
    #[derive(Deserialize)]
    struct Request {
        // identical, for back-compat (owners preferred)
        users: Option<Vec<String>>,
        owners: Option<Vec<String>>,
    }
    let request: Request =
        serde_json::from_reader(req.body_mut()).map_err(|_| cargo_err("invalid json request"))?;
    request
        .owners
        .or(request.users)
        .ok_or_else(|| cargo_err("invalid json request"))
}

fn modify_owners(req: &mut ConduitRequest, add: bool) -> AppResult<Json<Value>> {
    let crate_name = req.param("crate_id").unwrap();

    let auth = AuthCheck::default()
        .with_endpoint_scope(EndpointScope::ChangeOwners)
        .for_crate(crate_name)
        .check(req)?;

    let logins = parse_owners_request(req)?;
    let app = req.app();
    let crate_name = req.param("crate_id").unwrap();

    let conn = app.db_write()?;
    let user = auth.user();

    conn.transaction(|| {
        let krate: Crate = Crate::by_name(crate_name).first(&*conn)?;
        let owners = krate.owners(&conn)?;

        match user.rights(app, &owners)? {
            Rights::Full => {}
            // Yes!
            Rights::Publish => {
                return Err(cargo_err(
                    "team members don't have permission to modify owners",
                ));
            }
            Rights::None => {
                return Err(cargo_err("only owners have permission to modify owners"));
            }
        }

        let comma_sep_msg = if add {
            let mut msgs = Vec::with_capacity(logins.len());
            for login in &logins {
                let login_test =
                    |owner: &Owner| owner.login().to_lowercase() == *login.to_lowercase();
                if owners.iter().any(login_test) {
                    return Err(cargo_err(&format_args!("`{login}` is already an owner")));
                }
                let msg = krate.owner_add(app, &conn, &user, login)?;
                msgs.push(msg);
            }
            msgs.join(",")
        } else {
            for login in &logins {
                krate.owner_remove(app, &conn, &user, login)?;
            }
            if User::owning(&krate, &conn)?.is_empty() {
                return Err(cargo_err(
                    "cannot remove all individual owners of a crate. \
                     Team member don't have permission to modify owners, so \
                     at least one individual owner is required.",
                ));
            }
            "owners successfully removed".to_owned()
        };

        Ok(Json(json!({ "ok": true, "msg": comma_sep_msg })))
    })
}
