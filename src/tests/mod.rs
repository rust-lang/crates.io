use crate::util::{RequestHelper, TestApp, github::next_gh_id};
use crates_io::models::{NewCategory, NewTeam, NewUser};
use crates_io::views::{
    EncodableCategory, EncodableCrate, EncodableKeyword, EncodableOwner, EncodableVersion,
    GoodCrate,
};

use serde::{Deserialize, Serialize};

mod account_lock;
mod authentication;
mod blocked_routes;
pub use crates_io_test_utils::builders;
mod caching;
mod categories;
mod cors;
mod dump_db;
mod github_secret_scanning;
mod issues;
mod krate;
mod middleware;
mod not_found_error;
mod openapi;
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
    builders::UserBuilder::new().with_username(login).new_user()
}

fn new_team(login: &str) -> NewTeam<'_> {
    NewTeam::builder()
        .login(login)
        .org_id(next_gh_id())
        .github_id(next_gh_id())
        .build()
}

pub use crates_io_test_utils::helpers::add_team_to_crate;

fn new_category<'a>(category: &'a str, slug: &'a str, description: &'a str) -> NewCategory<'a> {
    NewCategory {
        category,
        slug,
        description,
    }
}
