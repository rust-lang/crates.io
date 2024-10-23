use crate::models::ApiToken;
use crate::schema::api_tokens;
use crate::tests::util::{RequestHelper, TestApp};
use diesel::prelude::*;
use diesel_async::RunQueryDsl;

#[derive(Deserialize)]
pub struct RevokedResponse {}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_token_non_existing() {
    let (_, _, user) = TestApp::init().with_user();
    let _json: RevokedResponse = user.delete("/api/v1/me/tokens/5").await.good();
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_token_doesnt_revoke_other_users_token() {
    let (app, _, user1, token) = TestApp::init().with_token();
    let mut conn = app.async_db_conn().await;
    let user1 = user1.as_model();
    let token = token.as_model();
    let user2 = app.db_new_user("baz");

    // List tokens for first user contains the token
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user1)
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, token.name);

    // Try revoke the token as second user
    let _json: RevokedResponse = user2
        .delete(&format!("/api/v1/me/tokens/{}", token.id))
        .await
        .good();

    // List tokens for first user still contains the token
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user1)
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, token.name);
}

#[tokio::test(flavor = "multi_thread")]
async fn revoke_token_success() {
    let (app, _, user, token) = TestApp::init().with_token();
    let mut conn = app.async_db_conn().await;

    // List tokens contains the token
    let tokens: Vec<ApiToken> = assert_ok!(
        ApiToken::belonging_to(user.as_model())
            .select(ApiToken::as_select())
            .load(&mut conn)
            .await
    );
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, token.as_model().name);

    // Revoke the token
    let _json: RevokedResponse = user
        .delete(&format!("/api/v1/me/tokens/{}", token.as_model().id))
        .await
        .good();

    // List tokens no longer contains the token
    let count = ApiToken::belonging_to(user.as_model())
        .filter(api_tokens::revoked.eq(false))
        .count()
        .get_result(&mut conn)
        .await;
    assert_eq!(count, Ok(0));
}
