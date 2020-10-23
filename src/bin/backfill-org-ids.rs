// Queries GitHub and backfills organization IDs for all teams without one
//
// Usage:
//      cargo run --bin backfill-org-ids

#![warn(clippy::all, rust_2018_idioms)]

use cargo_registry::{db, models::Team, schema::teams};

use diesel::prelude::*;
use regex::Regex;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use std::{env, thread, time};

fn main() {
    let conn = db::connect_now().unwrap();
    let client = Client::new();
    backfill_org_ids(&conn, &client);
}

fn backfill_org_ids(conn: &PgConnection, client: &Client) {
    let teams = teams::table
        .filter(teams::org_id.is_null())
        .load::<Team>(conn)
        .unwrap();

    let mut found = 0;
    for team in &teams {
        let option_org_id = query_for_org_id(client, team);
        match option_org_id {
            Some(org_id) => {
                found += 1;
                let update_result = diesel::update(teams::table)
                    .filter(teams::github_id.eq(team.github_id))
                    .set(teams::org_id.eq(org_id))
                    .execute(conn);
                if let Err(msg) = update_result {
                    println!(
                        "problem when updating record for team '{}': {}",
                        team.login, msg
                    );
                }
            }
            None => println!("could not find org id for team '{}'", team.login),
        }
        thread::sleep(time::Duration::from_secs(1));
    }

    println!(
        "Recorded organization ids for {} of {} teams",
        found,
        teams.len()
    );
}

fn query_for_org_id(client: &Client, team: &Team) -> Option<i32> {
    query_using_team_id(client, team).or_else(|| query_using_org_name(client, team))
}

fn query_using_team_id(client: &Client, team: &Team) -> Option<i32> {
    // GET https://api.github.com/teams/2874034
    let res = github_http_get(
        client,
        &format!("https://api.github.com/teams/{}", team.github_id),
    )?;

    // HTTP/1.1 404 Not Found
    // link: <https://docs.github.com/changes/2020-01-21-moving-the-team-api-endpoints/>; rel="deprecation"; type="text/html", <https://api.github.com/organizations/14631425/team/2874034>; rel="alternate"
    let link = res.headers().get("Link")?.to_str().ok()?;
    let links = parse_link_header::parse(link).ok()?;
    let re = Regex::new(r"/organizations/(\d+)/team").ok()?;
    let re_captures = re.captures(&links.get(&Some("alternate".into()))?.raw_uri)?;
    let org_id_str = re_captures.get(1)?.as_str();
    let org_id = i32::from_str_radix(&org_id_str, 10).ok()?;

    if org_id == 0 {
        None
    } else {
        Some(org_id)
    }
}

fn query_using_org_name(client: &Client, team: &Team) -> Option<i32> {
    let re = Regex::new(r":([^:]+):").unwrap();
    let org_name = re.captures(&team.login)?.get(1)?.as_str();

    // GET https://api.github.com/orgs/rust-lang-nursery
    let res = github_http_get(client, &format!("https://api.github.com/orgs/{}", org_name))?;
    if !res.status().is_success() {
        return None;
    }

    #[derive(Deserialize)]
    struct GithubOrganization {
        id: i32,
    }

    let org = res.json::<GithubOrganization>().ok()?;

    Some(org.id)
}

fn github_http_get(client: &Client, url: &str) -> Option<Response> {
    let gh_client_id = env::var("GH_CLIENT_ID").expect("must set GH_CLIENT_ID");
    let gh_client_secret = env::var("GH_CLIENT_SECRET").expect("must set GH_CLIENT_SECRET");

    let resp_result = client
        .get(url)
        .header(reqwest::header::ACCEPT, "application/vnd.github.v3+json")
        .header(reqwest::header::USER_AGENT, "crates.io (https://crates.io)")
        .basic_auth(&gh_client_id, Some(&gh_client_secret))
        .send();

    resp_result.ok()
}
