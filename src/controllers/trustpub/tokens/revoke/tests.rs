use crate::tests::util::{MockTokenUser, RequestHelper, TestApp};
use chrono::{TimeDelta, Utc};
use crates_io_database::models::trustpub::NewToken;
use crates_io_database::schema::trustpub_tokens;
use crates_io_trustpub::access_token::AccessToken;
use diesel::prelude::*;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use insta::assert_compact_debug_snapshot;
use insta::assert_snapshot;
use secrecy::ExposeSecret;
use sha2::Sha256;
use sha2::digest::Output;

const URL: &str = "/api/v1/trusted_publishing/tokens";

fn generate_token() -> (String, Output<Sha256>) {
    let token = AccessToken::generate();
    (token.finalize().expose_secret().to_string(), token.sha256())
}

async fn new_token(conn: &mut AsyncPgConnection, crate_id: i32) -> QueryResult<String> {
    let (token, hashed_token) = generate_token();

    let new_token = NewToken {
        expires_at: Utc::now() + TimeDelta::minutes(30),
        hashed_token: hashed_token.as_slice(),
        crate_ids: &[crate_id],
    };

    new_token.insert(conn).await?;

    Ok(token)
}

async fn all_crate_ids(conn: &mut AsyncPgConnection) -> QueryResult<Vec<Vec<Option<i32>>>> {
    trustpub_tokens::table
        .select(trustpub_tokens::crate_ids)
        .load(conn)
        .await
}

#[tokio::test(flavor = "multi_thread")]
async fn test_happy_path() -> anyhow::Result<()> {
    let (app, _client) = TestApp::full().empty().await;
    let mut conn = app.db_conn().await;

    let token1 = new_token(&mut conn, 1).await?;
    let _token2 = new_token(&mut conn, 2).await?;
    assert_compact_debug_snapshot!(all_crate_ids(&mut conn).await?, @"[[Some(1)], [Some(2)]]");

    let header = format!("Bearer {}", token1);
    let token_client = MockTokenUser::with_auth_header(header, app.clone());

    let response = token_client.delete::<()>(URL).await;
    assert_snapshot!(response.status(), @"204 No Content");
    assert_eq!(response.text(), "");

    // Check that the token is deleted
    assert_compact_debug_snapshot!(all_crate_ids(&mut conn).await?, @"[[Some(2)]]");

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_missing_authorization_header() -> anyhow::Result<()> {
    let (_app, client) = TestApp::full().empty().await;

    let response = client.delete::<()>(URL).await;
    assert_snapshot!(response.status(), @"401 Unauthorized");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Missing authorization header"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_authorization_header_format() -> anyhow::Result<()> {
    let (app, _client) = TestApp::full().empty().await;

    // Create a client with an invalid authorization header (missing "Bearer " prefix)
    let header = "invalid-format".to_string();
    let token_client = MockTokenUser::with_auth_header(header, app.clone());

    let response = token_client.delete::<()>(URL).await;
    assert_snapshot!(response.status(), @"401 Unauthorized");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid authorization header"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_invalid_token_format() -> anyhow::Result<()> {
    let (app, _client) = TestApp::full().empty().await;

    // Create a client with an invalid token format
    let header = "Bearer invalid-token".to_string();
    let token_client = MockTokenUser::with_auth_header(header, app.clone());

    let response = token_client.delete::<()>(URL).await;
    assert_snapshot!(response.status(), @"401 Unauthorized");
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"Invalid authorization header"}]}"#);

    Ok(())
}

#[tokio::test(flavor = "multi_thread")]
async fn test_non_existent_token() -> anyhow::Result<()> {
    let (app, _client) = TestApp::full().empty().await;

    // Generate a valid token format, but it doesn't exist in the database
    let (token, _) = generate_token();
    let header = format!("Bearer {}", token);
    let token_client = MockTokenUser::with_auth_header(header, app.clone());

    // The request should succeed with 204 No Content even though the token doesn't exist
    let response = token_client.delete::<()>(URL).await;
    assert_snapshot!(response.status(), @"204 No Content");
    assert_eq!(response.text(), "");

    Ok(())
}
