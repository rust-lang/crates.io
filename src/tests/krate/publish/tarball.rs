use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};
use crates_io_tarball::TarballBuilder;
use http::StatusCode;
use insta::assert_json_snapshot;

#[test]
fn new_krate_wrong_files() {
    let (app, _, user) = TestApp::full().with_user();
    let data: &[u8] = &[1];
    let files = [("foo-1.0.0/a", data), ("bar-1.0.0/a", data)];
    let builder = PublishBuilder::new("foo", "1.0.0").files(&files);

    let response = user.publish_crate(builder);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "invalid path found: bar-1.0.0/a" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn new_krate_tarball_with_hard_links() {
    let (app, _, _, token) = TestApp::full().with_token();

    let tarball = {
        let mut builder = TarballBuilder::new("foo", "1.1.0");

        let mut header = tar::Header::new_gnu();
        assert_ok!(header.set_path("foo-1.1.0/bar"));
        header.set_size(0);
        header.set_entry_type(tar::EntryType::hard_link());
        assert_ok!(header.set_link_name("foo-1.1.0/another"));
        header.set_cksum();
        assert_ok!(builder.as_mut().append(&header, &[][..]));

        builder.build()
    };

    let crate_to_publish = PublishBuilder::new("foo", "1.1.0").tarball(tarball);

    let response = token.publish_crate(crate_to_publish);
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.into_json(),
        json!({ "errors": [{ "detail": "unexpected symlink or hard link found: foo-1.1.0/bar" }] })
    );

    assert!(app.stored_files().is_empty());
}

#[test]
fn empty() {
    let (app, _, user) = TestApp::full().with_user();

    let response = user.put::<()>("/api/v1/crates/new", &[] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}

#[test]
fn json_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.put::<()>("/api/v1/crates/new", &[0u8, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}

#[test]
fn json_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.put::<()>("/api/v1/crates/new", &[100u8, 0, 0, 0, 0] as &[u8]);
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}

#[test]
fn tarball_len_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.put::<()>(
        "/api/v1/crates/new",
        &[2, 0, 0, 0, b'{', b'}', 0, 0] as &[u8],
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}

#[test]
fn tarball_bytes_truncated() {
    let (app, _, _, token) = TestApp::full().with_token();

    let response = token.put::<()>(
        "/api/v1/crates/new",
        &[2, 0, 0, 0, b'{', b'}', 100, 0, 0, 0, 0] as &[u8],
    );
    assert_eq!(response.status(), StatusCode::OK);
    assert_json_snapshot!(response.into_json());
    assert!(app.stored_files().is_empty());
}
