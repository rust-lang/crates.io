use crate::builders::CrateBuilder;
use crate::prelude::*;
use diesel::prelude::*;

#[test]
fn can_hit_read_only_endpoints_in_read_only_mode() -> CargoResult<()> {
    let app = App::new();
    app.set_read_only()?;
    app.get("/api/v1/crates").send()?;
    Ok(())
}

#[test]
fn cannot_hit_endpoint_which_writes_db_in_read_only_mode() -> CargoResult<()> {
    let app = App::new();
    let user = app.create_user("new_user")?;
    let token = app.token_for(&user)?;

    app.db(|conn| {
        CrateBuilder::new("foo_yank_read_only", user.id)
            .version("1.0.0")
            .build(conn)
    })?;
    app.set_read_only()?;
    let resp = app
        .delete("/api/v1/crates/foo_yank_read_only/1.0.0/yank")
        .with_token(&token)
        .send()
        .allow_errors()?;
    assert_eq!(503, resp.status());
    Ok(())
}

#[test]
fn can_download_crate_in_read_only_mode() -> CargoResult<()> {
    let app = App::new();

    app.db(|conn| {
        let user = app.create_user("new_user")?;
        CrateBuilder::new("foo_download_read_only", user.id)
            .version("1.0.0")
            .build(conn)
    })?;
    app.set_read_only()?;

    let resp = app
        .get("/api/v1/crates/foo_download_read_only/1.0.0/download")
        .send()?;
    assert_eq!(302, resp.status());

    // We're in read only mode so the download should not have been counted
    app.db(|conn| {
        use cargo_registry::schema::version_downloads::dsl::*;
        use diesel::dsl::sum;

        let dl_count = version_downloads
            .select(sum(downloads))
            .get_result::<Option<i64>>(conn)?;
        assert_eq!(None, dl_count);
        Ok(())
    })
}
