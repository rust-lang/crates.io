use diesel::prelude::*;
use std::collections::HashSet;

use conduit::{Handler, Method};

use cargo_registry::token::{ApiToken, EncodableApiTokenWithToken};

#[derive(Deserialize)]
struct DecodableApiToken {
    name: String,
}

#[derive(Deserialize)]
struct ListResponse {
    api_tokens: Vec<DecodableApiToken>,
}
#[derive(Deserialize)]
struct NewResponse {
    api_token: EncodableApiTokenWithToken,
}
#[derive(Deserialize)]
struct RevokedResponse {}

macro_rules! assert_contains {
    ($e:expr, $f:expr) => {
        if !$e.contains($f) {
            panic!(format!("expected '{}' to contain '{}'", $e, $f));
        }
    }
}

#[test]
fn list_logged_out() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me/tokens");

    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);
}

#[test]
fn list_empty() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me/tokens");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: ListResponse = ::json(&mut response);

    assert_eq!(json.api_tokens.len(), 0);
}

#[test]
fn list_tokens() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me/tokens");

    let (user, tokens);
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        tokens = vec![
            t!(ApiToken::insert(&conn, user.id, "bar")),
            t!(ApiToken::insert(&conn, user.id, "baz")),
        ];
    }
    ::sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: ListResponse = ::json(&mut response);

    assert_eq!(json.api_tokens.len(), tokens.len());
    assert_eq!(
        json.api_tokens
            .into_iter()
            .map(|t| t.name)
            .collect::<HashSet<_>>(),
        tokens.into_iter().map(|t| t.name).collect::<HashSet<_>>()
    );
}

#[test]
fn create_token_logged_out() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    req.with_body(br#"{ "api_token": { "name": "bar" } }"#);

    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);
}

#[test]
fn create_token_invalid_request() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);
    req.with_body(br#"{ "name": "" }"#);

    let mut response = t_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);

    assert_eq!(response.status.0, 400);
    assert_contains!(json.errors[0].detail, "invalid new token request");
}

#[test]
fn create_token_no_name() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);
    req.with_body(br#"{ "api_token": { "name": "" } }"#);

    let mut response = t_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);

    assert_eq!(response.status.0, 400);
    assert_eq!(json.errors[0].detail, "name must have a value");
}

#[test]
fn create_token_long_body() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);
    req.with_body(&[5; 5192]); // Send a request with a 5kB body of 5's

    let mut response = t_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);

    assert_eq!(response.status.0, 400);
    assert_contains!(json.errors[0].detail, "max post size");
}

#[test]
fn create_token_exceeded_tokens_per_user() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let user;
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        for i in 0..1000 {
            t!(ApiToken::insert(&conn, user.id, &format!("token {}", i)));
        }
    };
    ::sign_in_as(&mut req, &user);
    req.with_body(br#"{ "api_token": { "name": "bar" } }"#);

    let mut response = t_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);

    assert_eq!(response.status.0, 400);
    assert_contains!(json.errors[0].detail, "maximum tokens per user");
}

#[test]
fn create_token_success() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);
    req.with_body(br#"{ "api_token": { "name": "bar" } }"#);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: NewResponse = ::json(&mut response);

    assert_eq!(json.api_token.name, "bar");
    assert!(!json.api_token.token.is_empty());

    let conn = t!(app.diesel_database.get());
    let tokens = t!(ApiToken::belonging_to(&user).load::<ApiToken>(&*conn));
    assert_eq!(tokens.len(), 1);
    assert_eq!(tokens[0].name, "bar");
    assert_eq!(tokens[0].token, json.api_token.token);
    assert_eq!(tokens[0].last_used_at, None);
}

#[test]
fn create_token_multiple_have_different_values() {
    let (_b, app, middle) = ::app();

    let user = {
        let conn = t!(app.clone().diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };

    let first = {
        let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");
        ::sign_in_as(&mut req, &user);
        req.with_body(br#"{ "api_token": { "name": "bar" } }"#);
        ::json::<NewResponse>(&mut ok_resp!(middle.call(&mut req)))
    };

    let second = {
        let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");
        ::sign_in_as(&mut req, &user);
        req.with_body(br#"{ "api_token": { "name": "bar" } }"#);
        ::json::<NewResponse>(&mut ok_resp!(middle.call(&mut req)))
    };

    assert_ne!(first.api_token.token, second.api_token.token);
}

#[test]
fn create_token_multiple_users_have_different_values() {
    let (_b, app, middle) = ::app();

    let first_user = {
        let conn = t!(app.clone().diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };

    let second_user = {
        let conn = t!(app.clone().diesel_database.get());
        t!(::new_user("bar").create_or_update(&conn))
    };

    let first_token = {
        let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");
        ::sign_in_as(&mut req, &first_user);
        req.with_body(br#"{ "api_token": { "name": "baz" } }"#);
        ::json::<NewResponse>(&mut ok_resp!(middle.call(&mut req)))
    };

    let second_token = {
        let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");
        ::sign_in_as(&mut req, &second_user);
        req.with_body(br#"{ "api_token": { "name": "baz" } }"#);
        ::json::<NewResponse>(&mut ok_resp!(middle.call(&mut req)))
    };

    assert_ne!(first_token.api_token.token, second_token.api_token.token);
}

#[test]
fn create_token_with_token() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Post, "/api/v1/me/tokens");

    let (user, token);
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        token = t!(ApiToken::insert(&conn, user.id, "bar"));
    }
    req.header("Authorization", &token.token);
    req.with_body(br#"{ "api_token": { "name": "baz" } }"#);

    let mut response = t_resp!(middle.call(&mut req));
    let json: ::Bad = ::json(&mut response);

    assert_eq!(response.status.0, 400);
    assert_contains!(
        json.errors[0].detail,
        "cannot use an API token to create a new API token"
    );
}

#[test]
fn revoke_token_non_existing() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Delete, "/api/v1/me/tokens/5");

    let user = {
        let conn = t!(app.diesel_database.get());
        t!(::new_user("foo").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user);

    let mut response = ok_resp!(middle.call(&mut req));
    ::json::<RevokedResponse>(&mut response);
}

#[test]
fn revoke_token_doesnt_revoke_other_users_token() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Delete, "/api/v1/me/tokens");

    // Create one user with a token and sign in with a different user
    let (user1, token, user2);
    {
        let conn = t!(app.diesel_database.get());
        user1 = t!(::new_user("foo").create_or_update(&conn));
        token = t!(ApiToken::insert(&conn, user1.id, "bar"));
        user2 = t!(::new_user("baz").create_or_update(&conn))
    };
    ::sign_in_as(&mut req, &user2);

    // List tokens for first user contains the token
    {
        let conn = t!(app.diesel_database.get());
        let tokens = t!(ApiToken::belonging_to(&user1).load::<ApiToken>(&*conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.name);
    }

    // Try revoke the token as second user
    {
        req.with_path(&format!("/api/v1/me/tokens/{}", token.id));

        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<RevokedResponse>(&mut response);
    }

    // List tokens for first user still contains the token
    {
        let conn = t!(app.diesel_database.get());
        let tokens = t!(ApiToken::belonging_to(&user1).load::<ApiToken>(&*conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.name);
    }
}

#[test]
fn revoke_token_success() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Delete, "/api/v1/me/tokens");

    let (user, token);
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        token = t!(ApiToken::insert(&conn, user.id, "bar"));
    }
    ::sign_in_as(&mut req, &user);

    // List tokens contains the token
    {
        let conn = t!(app.diesel_database.get());
        let tokens = t!(ApiToken::belonging_to(&user).load::<ApiToken>(&*conn));
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].name, token.name);
    }

    // Revoke the token
    {
        req.with_path(&format!("/api/v1/me/tokens/{}", token.id));

        let mut response = ok_resp!(middle.call(&mut req));
        ::json::<RevokedResponse>(&mut response);
    }

    // List tokens no longer contains the token
    {
        let conn = t!(app.diesel_database.get());
        let tokens = ApiToken::belonging_to(&user).count().get_result(&*conn);
        assert_eq!(tokens, Ok(0));
    }
}

#[test]
fn token_gives_access_to_me() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");

    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    let (user, token);
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        token = t!(ApiToken::insert(&conn, user.id, "bar"));
    }
    req.header("Authorization", &token.token);

    let mut response = ok_resp!(middle.call(&mut req));
    let json: ::user::UserShowPrivateResponse = ::json(&mut response);

    assert_eq!(json.user.email, user.email);
}

#[test]
fn using_token_updates_last_used_at() {
    let (_b, app, middle) = ::app();
    let mut req = ::req(app.clone(), Method::Get, "/api/v1/me");
    let response = t_resp!(middle.call(&mut req));
    assert_eq!(response.status.0, 403);

    let (user, token);
    {
        let conn = t!(app.diesel_database.get());
        user = t!(::new_user("foo").create_or_update(&conn));
        token = t!(ApiToken::insert(&conn, user.id, "bar"));
    }
    req.header("Authorization", &token.token);
    assert!(token.last_used_at.is_none());

    ok_resp!(middle.call(&mut req));

    let token = {
        let conn = t!(app.diesel_database.get());
        t!(ApiToken::belonging_to(&user).first::<ApiToken>(&*conn))
    };
    assert!(token.last_used_at.is_some());

    // Would check that it updates the timestamp here, but the timestamp is
    // based on the start of the database transaction so it doesn't work in
    // this test framework.
}
