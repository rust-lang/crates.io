use crate::builders::CrateBuilder;
use crate::{RequestHelper, TestApp};
use diesel::prelude::*;

#[test]
fn can_hit_read_only_endpoints_in_read_only_mode() {
    let (app, anon) = TestApp::init().empty();
    app.db(set_read_only).unwrap();
    anon.get::<()>("/api/v1/crates").assert_status(200);
}

#[test]
fn cannot_hit_endpoint_which_writes_db_in_read_only_mode() {
    let (app, _, user, token) = TestApp::init().with_token();
    app.db(|conn| {
        CrateBuilder::new("foo_yank_read_only", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
        set_read_only(conn).unwrap();
    });
    token
        .delete::<()>("/api/v1/crates/foo_yank_read_only/1.0.0/yank")
        .assert_status(503);

    // Restore the transaction so `TestApp::drop` can still access the transaction
    app.db(|conn| {
        diesel::sql_query("ROLLBACK TO test_post_readonly")
            .execute(conn)
            .unwrap();
    });
}

#[test]
fn can_download_crate_in_read_only_mode() {
    let (app, anon, user) = TestApp::init().with_user();

    app.db(|conn| {
        CrateBuilder::new("foo_download_read_only", user.as_model().id)
            .version("1.0.0")
            .expect_build(conn);
        set_read_only(conn).unwrap();
    });

    anon.get::<()>("/api/v1/crates/foo_download_read_only/1.0.0/download")
        .assert_status(302);

    // We're in read only mode so the download should not have been counted
    app.db(|conn| {
        use cargo_registry::schema::version_downloads::dsl::*;
        use diesel::dsl::sum;

        let dl_count = version_downloads
            .select(sum(downloads))
            .get_result::<Option<i64>>(conn);
        assert_eq!(Ok(None), dl_count);
    })
}

fn set_read_only(conn: &PgConnection) -> QueryResult<()> {
    diesel::sql_query("SET TRANSACTION READ ONLY").execute(conn)?;
    diesel::sql_query("SAVEPOINT test_post_readonly").execute(conn)?;
    Ok(())
}
