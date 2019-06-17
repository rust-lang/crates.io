//! All routes related to managing owners of a crate

use serde_json;

use crate::controllers::prelude::*;
use crate::models::{Crate, Owner, Rights, Team, User};
use crate::views::EncodableOwner;

/// Handles the `GET /crates/:crate_id/owners` route.
pub fn owners(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = krate
        .owners(&conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        users: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { users: owners }))
}

/// Handles the `GET /crates/:crate_id/owner_team` route.
pub fn owner_team(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = Team::owning(&krate, &conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        teams: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { teams: owners }))
}

/// Handles the `GET /crates/:crate_id/owner_user` route.
pub fn owner_user(req: &mut dyn Request) -> CargoResult<Response> {
    let crate_name = &req.params()["crate_id"];
    let conn = req.db_conn()?;
    let krate = Crate::by_name(crate_name).first::<Crate>(&*conn)?;
    let owners = User::owning(&krate, &conn)?
        .into_iter()
        .map(Owner::encodable)
        .collect();

    #[derive(Serialize)]
    struct R {
        users: Vec<EncodableOwner>,
    }
    Ok(req.json(&R { users: owners }))
}

/// Handles the `PUT /crates/:crate_id/owners` route.
pub fn add_owners(req: &mut dyn Request) -> CargoResult<Response> {
    modify_owners(req, true)
}

/// Handles the `DELETE /crates/:crate_id/owners` route.
pub fn remove_owners(req: &mut dyn Request) -> CargoResult<Response> {
    modify_owners(req, false)
}

fn modify_owners(req: &mut dyn Request, add: bool) -> CargoResult<Response> {
    let mut body = String::new();
    req.body().read_to_string(&mut body)?;

    let user = req.user()?;
    let conn = req.db_conn()?;
    let krate = Crate::by_name(&req.params()["crate_id"]).first::<Crate>(&*conn)?;
    let owners = krate.owners(&conn)?;

    match user.rights(req.app(), &owners)? {
        Rights::Full => {}
        // Yes!
        Rights::Publish => {
            return Err(human("team members don't have permission to modify owners"));
        }
        Rights::None => {
            return Err(human("only owners have permission to modify owners"));
        }
    }

    #[derive(Deserialize)]
    struct Request {
        // identical, for back-compat (owners preferred)
        users: Option<Vec<String>>,
        owners: Option<Vec<String>>,
    }

    let request: Request =
        serde_json::from_str(&body).map_err(|_| human("invalid json request"))?;

    let logins = request
        .owners
        .or(request.users)
        .ok_or_else(|| human("invalid json request"))?;

    let mut msgs = Vec::new();

    for login in &logins {
        if add {
            let login_test = |owner: &Owner| owner.login().to_lowercase() == *login.to_lowercase();
            if owners.iter().any(login_test) {
                return Err(human(&format_args!("`{}` is already an owner", login)));
            }
            let msg = krate.owner_add(req.app(), &conn, user, login)?;
            msgs.push(msg);
        } else {
            // Removing the team that gives you rights is prevented because
            // team members only have Rights::Publish
            if owners.len() == 1 {
                return Err(human("cannot remove the sole owner of a crate"));
            }
            krate.owner_remove(req.app(), &conn, user, login)?;
        }
    }

    let comma_sep_msg = msgs.join(",");

    #[derive(Serialize)]
    struct R {
        ok: bool,
        msg: String,
    }
    Ok(req.json(&R {
        ok: true,
        msg: comma_sep_msg,
    }))
}
