use crate::tests::builders::{CrateBuilder, VersionBuilder};
use crate::tests::util::{RequestHelper, TestApp};
use crate::views::EncodableDependency;
use http::StatusCode;
use insta::assert_snapshot;

#[derive(Deserialize)]
pub struct Deps {
    pub dependencies: Vec<EncodableDependency>,
}

#[tokio::test(flavor = "multi_thread")]
async fn dependencies() {
    let (app, anon, user) = TestApp::init().with_user().await;
    let mut conn = app.async_db_conn().await;
    let user = user.as_model();

    let c1 = CrateBuilder::new("foo_deps", user.id)
        .expect_build(&mut conn)
        .await;
    let c2 = CrateBuilder::new("bar_deps", user.id)
        .expect_build(&mut conn)
        .await;
    VersionBuilder::new("1.0.0")
        .dependency(&c2, None)
        .expect_build(c1.id, user.id, &mut conn)
        .await;

    let deps: Deps = anon
        .get("/api/v1/crates/foo_deps/1.0.0/dependencies")
        .await
        .good();
    assert_eq!(deps.dependencies[0].crate_id, "bar_deps");

    let response = anon
        .get::<()>("/api/v1/crates/missing-crate/1.0.0/dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `missing-crate` does not exist"}]}"#);

    let response = anon
        .get::<()>("/api/v1/crates/foo_deps/1.0.2/dependencies")
        .await;
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
    assert_snapshot!(response.text(), @r#"{"errors":[{"detail":"crate `foo_deps` does not have a version `1.0.2`"}]}"#);
}
