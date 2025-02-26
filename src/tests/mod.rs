use crate::models::{Crate, CrateOwner, NewCategory, NewTeam, NewUser, Team, User};
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::{
    EncodableCategory, EncodableCrate, EncodableKeyword, EncodableOwner, EncodableVersion,
    GoodCrate,
};

use crate::tests::util::github::next_gh_id;
use diesel::prelude::*;
use diesel_async::AsyncPgConnection;

mod account_lock;
mod authentication;
mod blocked_routes;
pub mod builders;
mod categories;
mod cors;
mod dump_db;
mod github_secret_scanning;
mod issues;
mod krate;
mod middleware;
mod not_found_error;
mod owners;
mod pagination;
mod read_only_mode;
mod routes;
mod server;
mod team;
mod token;
mod unhealthy_database;
mod user;
pub mod util;
mod version;
mod worker;

#[derive(Deserialize)]
pub struct CrateList {
    crates: Vec<EncodableCrate>,
    meta: CrateMeta,
}
#[derive(Deserialize)]
struct CrateMeta {
    total: i32,
    next_page: Option<String>,
    prev_page: Option<String>,
}
#[derive(Deserialize)]
#[allow(dead_code)]
pub struct CrateResponse {
    #[serde(rename = "crate")]
    krate: EncodableCrate,
    versions: Option<Vec<EncodableVersion>>,
    keywords: Option<Vec<EncodableKeyword>>,
}
#[derive(Serialize, Deserialize)]
pub struct VersionResponse {
    version: EncodableVersion,
}
#[derive(Deserialize)]
pub struct OwnerTeamsResponse {
    teams: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
pub struct OwnersResponse {
    users: Vec<EncodableOwner>,
}
#[derive(Deserialize)]
pub struct CategoryResponse {
    category: EncodableCategory,
}
#[derive(Deserialize)]
pub struct CategoryListResponse {
    categories: Vec<EncodableCategory>,
    meta: CategoryMeta,
}
#[derive(Deserialize)]
pub struct CategoryMeta {
    total: i32,
}
#[derive(Deserialize)]
pub struct OkBool {
    #[allow(dead_code)]
    ok: bool,
}

#[derive(Deserialize, Debug)]
pub struct OwnerResp {
    // server must include `ok: true` to support old cargo clients
    ok: bool,
    msg: String,
}

fn new_user(login: &str) -> NewUser<'_> {
    NewUser::builder()
        .gh_id(next_gh_id())
        .gh_login(login)
        .gh_access_token("some random token")
        .build()
}

fn new_team(login: &str) -> NewTeam<'_> {
    NewTeam::builder()
        .login(login)
        .org_id(next_gh_id())
        .github_id(next_gh_id())
        .build()
}

pub async fn add_team_to_crate(
    t: &Team,
    krate: &Crate,
    u: &User,
    conn: &mut AsyncPgConnection,
) -> QueryResult<()> {
    CrateOwner::builder()
        .crate_id(krate.id)
        .team_id(t.id)
        .created_by(u.id)
        .build()
        .insert(conn)
        .await
}

fn new_category<'a>(category: &'a str, slug: &'a str, description: &'a str) -> NewCategory<'a> {
    NewCategory {
        category,
        slug,
        description,
    }
}
