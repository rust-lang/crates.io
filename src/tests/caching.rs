//! Tests for the HTTP caching headers that control CDN and browser caching.
//!
//! Responses that depend on the authenticated identity must carry
//! `Cache-Control: no-store` so that no shared cache (CDN) or browser cache
//! stores them. Identity can come from a session cookie or an API token, so
//! `no-store` is used instead of relying on `Vary: Cookie`.

use crate::builders::CrateBuilder;
use crate::util::{MockRequestExt, RequestHelper, TestApp};
use http::{StatusCode, header};

#[tokio::test(flavor = "multi_thread")]
async fn me_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/me").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn me_updates_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/me/updates").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn me_tokens_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/me/tokens").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn me_token_is_not_cached() {
    let (_, _, user, token) = TestApp::init().with_token().await;
    let url = format!("/api/v1/me/tokens/{}", token.as_model().id);
    let response = user.get::<()>(&url).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn me_crate_owner_invitations_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/me/crate_owner_invitations").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn private_crate_owner_invitations_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let id = user.as_model().id;
    let url = format!("/api/private/crate_owner_invitations?invitee_id={id}");
    let response = user.get::<()>(&url).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn trustpub_github_configs_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let id = user.as_model().id;
    let url = format!("/api/v1/trusted_publishing/github_configs?user_id={id}");
    let response = user.get::<()>(&url).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn trustpub_gitlab_configs_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let id = user.as_model().id;
    let url = format!("/api/v1/trusted_publishing/gitlab_configs?user_id={id}");
    let response = user.get::<()>(&url).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn crate_following_status_is_not_cached() {
    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    CrateBuilder::new("foo", user.as_model().id)
        .expect_build(&mut conn)
        .await;
    let response = user.get::<()>("/api/v1/crates/foo/following").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn search_with_following_is_not_cached() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/crates?following=yes").await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn admin_list_is_not_cached() {
    use crates_io::schema::users;
    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;

    let (app, _, user) = TestApp::init().with_user().await;
    let mut conn = app.db_conn().await;
    diesel::update(user.as_model())
        .set(users::is_admin.eq(true))
        .execute(&mut conn)
        .await
        .unwrap();

    let response = user.admin_list::<()>(&user.as_model().gh_login).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn metrics_is_not_cached() {
    let (_, anon) = TestApp::init()
        .with_config(|config| config.metrics.authorization_token = Some("secret".into()))
        .empty()
        .await;
    let mut request = anon.get_request("/api/private/metrics/service");
    request.header("Authorization", "Bearer secret");
    let response = anon.run::<()>(request).await;
    response.assert_cache_control("no-store");
}

#[tokio::test(flavor = "multi_thread")]
async fn search_without_following_is_cacheable() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon.get::<()>("/api/v1/crates").await;
    response.assert_no_cache_control();
}

#[tokio::test(flavor = "multi_thread")]
async fn download_varies_on_accept() {
    let (_, anon) = TestApp::init().empty().await;

    // The default `Accept` header redirects (302) to the crate file.
    let response = anon.get::<()>("/api/v1/crates/foo/1.0.0/download").await;
    assert_eq!(response.status(), StatusCode::FOUND);
    response.assert_redirect_ends_with("/crates/foo/foo-1.0.0.crate");
    response.assert_vary(&["accept"]);

    // `Accept: application/json` returns a 200 with the URL as JSON.
    let mut request = anon.get_request("/api/v1/crates/foo/1.0.0/download");
    request.header(header::ACCEPT, "application/json");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.json().get("url").is_some());
    response.assert_vary(&["accept", "accept-encoding"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn readme_varies_on_accept() {
    let (_, anon) = TestApp::init().empty().await;

    // The default `Accept` header redirects (302) to the rendered readme.
    let response = anon.get::<()>("/api/v1/crates/foo/1.0.0/readme").await;
    assert_eq!(response.status(), StatusCode::FOUND);
    response.assert_redirect_ends_with("/readmes/foo/foo-1.0.0.html");
    response.assert_vary(&["accept"]);

    let mut request = anon.get_request("/api/v1/crates/foo/1.0.0/readme");
    request.header(header::ACCEPT, "application/json");
    let response = anon.run::<()>(request).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.json().get("url").is_some());
    response.assert_vary(&["accept", "accept-encoding"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn public_endpoint_is_cacheable() {
    let (_, anon) = TestApp::init().empty().await;
    let response = anon.get::<()>("/api/v1/categories").await;
    response.assert_no_cache_control();
    // The only `Vary` value is `Accept-Encoding`, added by the compression
    // middleware.
    response.assert_vary(&["accept-encoding"]);
}

#[tokio::test(flavor = "multi_thread")]
async fn public_endpoint_is_cacheable_for_authenticated_users() {
    let (_, _, user) = TestApp::init().with_user().await;
    let response = user.get::<()>("/api/v1/categories").await;
    response.assert_no_cache_control();
}
