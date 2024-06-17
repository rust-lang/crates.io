use crate::util::{RequestHelper, TestApp};
use chrono::{Duration, Utc};
use crates_io::models::token::{CrateScope, EndpointScope};
use crates_io::models::ApiToken;
use crates_io::schema::api_tokens;
use diesel::prelude::*;
use http::StatusCode;
use insta::assert_json_snapshot;

#[tokio::test(flavor = "multi_thread")]
async fn show_token_non_existing() {
    let url = "/api/v1/me/tokens/10086";
    let (_, _, user, _) = TestApp::init().with_token();
    user.get(url).await.assert_not_found();
}

#[tokio::test(flavor = "multi_thread")]
async fn show() {
    let (_, _, user, token) = TestApp::init().with_token();
    let token = token.as_model();
    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.created_at" => "[datetime]",
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn show_token_with_scopes() {
    let (app, _, user) = TestApp::init().with_user();
    let user_model = user.as_model();
    let id = user_model.id;
    app.db(|conn| {
        vec![
            assert_ok!(ApiToken::insert(conn, id, "bar")),
            assert_ok!(ApiToken::insert_with_scopes(
                conn,
                id,
                "baz",
                Some(vec![
                    CrateScope::try_from("serde").unwrap(),
                    CrateScope::try_from("serde-*").unwrap()
                ]),
                Some(vec![EndpointScope::PublishUpdate]),
                Some((Utc::now() - Duration::days(31)).naive_utc()),
            )),
        ]
    });

    let token: ApiToken = app.db(|conn| {
        ApiToken::belonging_to(user_model)
            .filter(api_tokens::name.eq("baz"))
            .select(ApiToken::as_select())
            .first(conn)
            .unwrap()
    });

    let url = format!("/api/v1/me/tokens/{}", token.id);
    let response = user.get::<()>(&url).await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.json(), {
        ".api_token.created_at" => "[datetime]",
        ".api_token.expired_at" => "[datetime]",
    });
}
