//! Integration tests that run against the live Docker integration container.
//!
//! These tests hit the crates.io HTTP API at `localhost:9888` (the
//! `integration` service from docker-compose). They skip automatically
//! if the container isn't reachable, so `cargo test` works without
//! Docker running.
//!
//! Start the container before running:
//!
//!     docker compose up -d --wait integration
//!
//! Then run just these tests:
//!
//!     cargo test --test integration docker_integration

use reqwest::StatusCode;
use std::process::Command;

const BASE_URL: &str = "http://localhost:9888";
const SESSION_KEY_RAW: &str = "badkeyabcdefghijklmnopqrstuvwxyzabcdef";

/// Returns a client (with cookie store) if the integration container is
/// reachable, or None (causing the test to return early) if it isn't.
async fn try_connect() -> Option<reqwest::Client> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(3))
        .build()
        .ok()?;

    match client.get(format!("{BASE_URL}/api/v1/summary")).send().await {
        Ok(resp) if resp.status().is_success() => Some(client),
        _ => {
            eprintln!("  SKIP: integration container not reachable at {BASE_URL}");
            None
        }
    }
}

/// Seed a test user + oauth_github row in the integration container's DB.
/// Returns the user's database id.
fn seed_test_user(login: &str, gh_account_id: i64) -> Option<i32> {
    assert!(
        login.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "login must be alphanumeric/dash/underscore, got: {login}"
    );

    let insert_user = format!(
        "INSERT INTO users (gh_id, gh_login, gh_avatar, gh_encrypted_token, name) \
         VALUES ({gh_account_id}, '{login}', \
         'https://avatars.example.com/{login}', '\\x00', 'Test User {login}') \
         ON CONFLICT ((gh_id) WHERE gh_id > 0) DO UPDATE SET gh_login = EXCLUDED.gh_login \
         RETURNING id"
    );

    let output = Command::new("docker")
        .args([
            "exec", "cratesio-postgres-1", "psql", "-U", "postgres",
            "-d", "cargo_registry_test", "-t", "-A", "-c", &insert_user,
        ])
        .output()
        .ok()?;

    let user_id: i32 = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .ok()?;

    // Insert matching oauth_github row
    let insert_oauth = format!(
        "INSERT INTO oauth_github (account_id, user_id, login, avatar, encrypted_token) \
         VALUES ({gh_account_id}, {user_id}, '{login}', \
         'https://avatars.example.com/{login}', '\\x00') \
         ON CONFLICT (account_id) DO UPDATE SET login = EXCLUDED.login"
    );

    Command::new("docker")
        .args([
            "exec", "cratesio-postgres-1", "psql", "-U", "postgres",
            "-d", "cargo_registry_test", "-t", "-A", "-c", &insert_oauth,
        ])
        .output()
        .ok()?;

    Some(user_id)
}

/// Forge a signed session cookie for the given user id, using the same
/// SESSION_KEY the integration container uses.
fn forge_session_cookie(user_id: i32) -> String {
    let session_key = cookie::Key::derive_from(SESSION_KEY_RAW.as_bytes());
    crate::util::encode_session_header(&session_key, user_id)
}

// -- session::begin tests --------------------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn begin_defaults_to_github() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!("{BASE_URL}/api/private/session/begin"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    let url = body["url"].as_str().expect("missing url field");

    assert!(
        url.contains("github.com/login/oauth/authorize"),
        "expected GitHub OAuth URL, got: {url}"
    );
    assert!(
        url.contains("scope=read%3Aorg"),
        "expected read:org scope, got: {url}"
    );
    assert!(
        body["state"].as_str().is_some_and(|s| !s.is_empty()),
        "expected non-empty state"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn begin_with_explicit_github_provider() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!("{BASE_URL}/api/private/session/begin?provider=github"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    let url = body["url"].as_str().unwrap();
    assert!(url.contains("github.com/login/oauth/authorize"));
}

#[tokio::test(flavor = "multi_thread")]
async fn begin_with_unknown_provider_returns_404() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!(
            "{BASE_URL}/api/private/session/begin?provider=nosuchprovider"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}

// -- session::authorize error paths ----------------------------------------

#[tokio::test(flavor = "multi_thread")]
async fn authorize_without_session_returns_400() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!(
            "{BASE_URL}/api/private/session/authorize?code=bogus&state=bogus"
        ))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
}

// -- seeded user: verify GitHub identity via API ---------------------------

#[tokio::test(flavor = "multi_thread")]
async fn public_user_endpoint_shows_github_identity() {
    let Some(client) = try_connect().await else {
        return;
    };

    let Some(_user_id) = seed_test_user("test-octocat", 99001) else {
        eprintln!("  SKIP: couldn't seed test user via docker exec");
        return;
    };

    let resp = client
        .get(format!("{BASE_URL}/api/v1/users/test-octocat"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    let user = &body["user"];

    assert_eq!(user["login"].as_str(), Some("test-octocat"));
    assert_eq!(
        user["avatar"].as_str(),
        Some("https://avatars.example.com/test-octocat")
    );
    assert_eq!(user["name"].as_str(), Some("Test User test-octocat"));
}

#[tokio::test(flavor = "multi_thread")]
async fn authenticated_me_endpoint_shows_github_identity() {
    let Some(client) = try_connect().await else {
        return;
    };

    let Some(user_id) = seed_test_user("test-authed-user", 99002) else {
        eprintln!("  SKIP: couldn't seed test user via docker exec");
        return;
    };

    let cookie = forge_session_cookie(user_id);

    let resp = client
        .get(format!("{BASE_URL}/api/v1/me"))
        .header("cookie", &cookie)
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    let user = &body["user"];

    assert_eq!(user["login"].as_str(), Some("test-authed-user"));
    assert_eq!(
        user["avatar"].as_str(),
        Some("https://avatars.example.com/test-authed-user")
    );
    assert_eq!(user["name"].as_str(), Some("Test User test-authed-user"));
    // private fields present on /me
    assert!(user["is_admin"].is_boolean());
    assert!(user["publish_notifications"].is_boolean());
}

#[tokio::test(flavor = "multi_thread")]
async fn me_endpoint_rejects_unauthenticated() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!("{BASE_URL}/api/v1/me"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::FORBIDDEN);
}

#[tokio::test(flavor = "multi_thread")]
async fn user_not_found_returns_404() {
    let Some(client) = try_connect().await else {
        return;
    };

    let resp = client
        .get(format!("{BASE_URL}/api/v1/users/no-such-user-ever"))
        .send()
        .await
        .unwrap();

    assert_eq!(resp.status(), StatusCode::NOT_FOUND);
}
