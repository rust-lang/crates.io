use crate::tests::builders::{CrateBuilder, DependencyBuilder, PublishBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use googletest::prelude::*;
use http::StatusCode;
use insta::{assert_json_snapshot, assert_snapshot};

#[tokio::test(flavor = "multi_thread")]
async fn features_version_2() {
    let (app, _, user, token) = TestApp::full().with_token().await;
    let mut conn = app.db_conn().await;

    // Insert a crate directly into the database so that foo_new can depend on it
    CrateBuilder::new("bar", user.as_model().id)
        .expect_build(&mut conn)
        .await;

    let dependency = DependencyBuilder::new("bar");

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0")
        .dependency(dependency)
        .feature("new_feat", &["dep:bar", "bar?/feat"])
        .feature("old_feat", &[]);
    token.publish_crate(crate_to_publish).await.good();

    let crates = app.crates_from_index_head("foo");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn feature_name_with_dot() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("foo.bar", &[]);
    token.publish_crate(crate_to_publish).await.good();
    let crates = app.crates_from_index_head("foo");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn feature_name_start_with_number_and_underscore() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0")
        .feature("0foo1.bar", &[])
        .feature("_foo2.bar", &[]);
    token.publish_crate(crate_to_publish).await.good();
    let crates = app.crates_from_index_head("foo");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn feature_name_with_unicode_chars() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("foo.你好世界", &[]);
    token.publish_crate(crate_to_publish).await.good();
    let crates = app.crates_from_index_head("foo");
    assert_json_snapshot!(crates);
}

#[tokio::test(flavor = "multi_thread")]
async fn empty_feature_name() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("", &[]);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"feature cannot be empty"}]}"#);
    assert!(app.stored_files().await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_feature_name1() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("~foo", &[]);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `~` in feature `~foo`, the first character must be a Unicode XID start character or digit (most letters or `_` or `0` to `9`)"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_feature_name2() {
    let (app, _, _, token) = TestApp::full().with_token().await;

    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("foo", &["!bar"]);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `!` in feature `!bar`, the first character must be a Unicode XID start character or digit (most letters or `_` or `0` to `9`)"}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn invalid_feature_name_start_with_hyphen() {
    let (app, _, _, token) = TestApp::full().with_token().await;
    let crate_to_publish = PublishBuilder::new("foo", "1.0.0").feature("-foo1.bar", &[]);
    let response = token.publish_crate(crate_to_publish).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"invalid character `-` in feature `-foo1.bar`, the first character must be a Unicode XID start character or digit (most letters or `_` or `0` to `9`)"}]}"#);
    assert!(app.stored_files().await.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_features() {
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_features = 3;
        })
        .with_token()
        .await;

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .feature("one", &[])
        .feature("two", &[])
        .feature("three", &[])
        .feature("four", &[])
        .feature("five", &[]);
    let response = token.publish_crate(publish_builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crates.io only allows a maximum number of 3 features, but your crate is declaring 5 features.\n\nTake a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html to understand why this restriction was introduced.\n\nIf you have a use case that requires an increase of this limit, please send us an email to help@crates.io to discuss the details."}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_features_with_custom_limit() {
    let (app, _, user, token) = TestApp::full()
        .with_config(|config| {
            config.max_features = 3;
        })
        .with_token()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user.as_model().id)
        .max_features(4)
        .expect_build(&mut conn)
        .await;

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .feature("one", &[])
        .feature("two", &[])
        .feature("three", &[])
        .feature("four", &[])
        .feature("five", &[]);
    let response = token.publish_crate(publish_builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crates.io only allows a maximum number of 4 features, but your crate is declaring 5 features.\n\nTake a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html to understand why this restriction was introduced.\n\nIf you have a use case that requires an increase of this limit, please send us an email to help@crates.io to discuss the details."}]}"#);
    assert_that!(app.stored_files().await, empty());

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .feature("one", &[])
        .feature("two", &[])
        .feature("three", &[])
        .feature("four", &[]);
    token.publish_crate(publish_builder).await.good();

    // see https://github.com/rust-lang/crates.io/issues/7632
    let publish_builder = PublishBuilder::new("foo", "1.0.1")
        .feature("one", &[])
        .feature("two", &[])
        .feature("three", &[])
        .feature("four", &[]);
    token.publish_crate(publish_builder).await.good();
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_enabled_features() {
    let (app, _, _, token) = TestApp::full()
        .with_config(|config| {
            config.max_features = 3;
        })
        .with_token()
        .await;

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .feature("default", &["one", "two", "three", "four", "five"]);
    let response = token.publish_crate(publish_builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crates.io only allows a maximum number of 3 features or dependencies that another feature can enable, but the \"default\" feature of your crate is enabling 5 features or dependencies.\n\nTake a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html to understand why this restriction was introduced.\n\nIf you have a use case that requires an increase of this limit, please send us an email to help@crates.io to discuss the details."}]}"#);
    assert_that!(app.stored_files().await, empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn too_many_enabled_features_with_custom_limit() {
    let (app, _, user, token) = TestApp::full()
        .with_config(|config| {
            config.max_features = 3;
        })
        .with_token()
        .await;

    let mut conn = app.db_conn().await;

    CrateBuilder::new("foo", user.as_model().id)
        .max_features(4)
        .expect_build(&mut conn)
        .await;

    let publish_builder = PublishBuilder::new("foo", "1.0.0")
        .feature("default", &["one", "two", "three", "four", "five"]);
    let response = token.publish_crate(publish_builder).await;
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crates.io only allows a maximum number of 4 features or dependencies that another feature can enable, but the \"default\" feature of your crate is enabling 5 features or dependencies.\n\nTake a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html to understand why this restriction was introduced.\n\nIf you have a use case that requires an increase of this limit, please send us an email to help@crates.io to discuss the details."}]}"#);
    assert_that!(app.stored_files().await, empty());

    let publish_builder =
        PublishBuilder::new("foo", "1.0.0").feature("default", &["one", "two", "three", "four"]);
    token.publish_crate(publish_builder).await.good();
}
