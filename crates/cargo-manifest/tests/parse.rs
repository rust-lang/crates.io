use cargo_manifest as lib;
use cargo_manifest::{Manifest, MaybeInherited};
use std::fs::read;
use std::str::FromStr;

mod utils;

#[test]
fn own() {
    let m = Manifest::from_slice(&read("Cargo.toml").unwrap()).unwrap();
    let package = m.package.as_ref().unwrap();
    assert_eq!("cargo-manifest", package.name);
    let m =
        Manifest::<toml::Value>::from_slice_with_metadata(&read("Cargo.toml").unwrap()).unwrap();
    let package = m.package.as_ref().unwrap();
    assert_eq!("cargo-manifest", package.name);
    assert_eq!(
        Some(MaybeInherited::Local(lib::Edition::E2021)),
        package.edition
    );
}

#[test]
fn opt_level() {
    let m = Manifest::from_slice(&read("tests/opt_level.toml").unwrap()).unwrap();
    insta::assert_debug_snapshot!(m);
}

#[test]
fn opt_version() {
    let m = Manifest::from_path("tests/opt_version.toml").expect("load metadata");
    insta::assert_debug_snapshot!(m);
}

#[test]
fn autobuild() {
    let manifest = r#"
    [package]
    name = "buildrstest"
    version = "0.2.0"
    "#;
    let tempdir = utils::prepare(manifest, vec!["build.rs"]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
}

/// Checks that explicit `build` key in Cargo.toml has precedence over auto-detected build.rs file.
#[test]
fn metadata() {
    let manifest = r#"
    [package]
    name = "metadata"
    version = "0.1.0"
    build = "foobar.rs"

    [lib]
    path = "lib.rs"
    "#;
    let tempdir = utils::prepare(manifest, vec!["build.rs", "lib.rs"]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
}

#[test]
fn readme() {
    let base = "[package]\nname = \"foo\"\nversion = \"1\"";

    let m = Manifest::from_str(&format!("{}\nreadme = \"hello.md\"", base)).unwrap();
    let readme = m.package.unwrap().readme.unwrap();
    assert_eq!(
        MaybeInherited::Local(lib::StringOrBool::String("hello.md".to_string())),
        readme
    );

    let m = Manifest::from_str(&format!("{}\nreadme = true", base)).unwrap();
    let readme = m.package.unwrap().readme.unwrap();
    assert_eq!(MaybeInherited::Local(lib::StringOrBool::Bool(true)), readme);

    let m = Manifest::from_str(&format!("{}\nreadme = 1", base));
    assert!(m.is_err());
}

#[test]
fn legacy() {
    let m = Manifest::from_slice(
        br#"[project]
                name = "foo"
                version = "1"
                "#,
    )
    .expect("parse old");
    insta::assert_debug_snapshot!(m);

    let m = Manifest::from_str("name = \"foo\"\nversion=\"1\"").expect("parse bare");
    insta::assert_debug_snapshot!(m);
}

// -- Multi-word identifiers can be specified using both snake_case and kebab-case --

/// This test ensures that the snake_case variant is handled correctly for `default-features`
#[test]
fn default_features_casing() {
    let m = Manifest::from_str(
        r#"
[package]
name = "foo"
version = "1"

[dependencies]
rusoto_core = { version = "0.45.0", default_features=false, features=["rustls"] }
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}

/// This test ensures that the snake_case variant is handled correctly for `build-dependencies`
#[test]
fn build_dependencies_casing() {
    let m = Manifest::from_str(
        r#"
[package]
name = "foo"
version = "1"

[build_dependencies]
lazy_static = "1.4.0"
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}

/// This test ensures that the snake_case variant is handled correctly for `dev-dependencies`
#[test]
fn dev_dependencies_casing() {
    let m = Manifest::from_str(
        r#"
[package]
name = "foo"
version = "1"

[dev_dependencies]
lazy_static = "1.4.0"
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}

/// This test ensures that both the kebap-case and the snake_case variant is handled correctly for `proc-macro`
#[test]
fn proc_macro_casing() {
    let m = Manifest::from_str(
        r#"
[package]
name = "foo"
version = "1"

[lib]
proc-macro = true
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);

    let m = Manifest::from_str(
        r#"
[package]
name = "foo"
version = "1"

[lib]
proc_macro = true
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}

/// We can work with package properties inherited from the workspace manifest.
#[test]
fn package_inheritance() {
    let m = Manifest::from_str(
        r#"
[package]
name = "bar"
version.workspace = true
authors.workspace = true
description.workspace = true
documentation.workspace = true
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}

/// This test ensures that we correctly handle crate dependencies that are deferred to the top-level workspace.
#[test]
fn workspace_dependency() {
    let m = Manifest::from_str(
        r#"
[workspace]
members = ["core"]
[workspace.dependencies]
chrono = "0.4"
serde = { version = "1.0", features = [ "derive" ] }
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);

    let m = Manifest::from_str(
        r#"
[package]
name = "core"
version = "0.1.0"
[dependencies]
chrono.workspace = true
config = "0.13"
tokio = { workspace = true, features = [ "rt-multi-thread" ] }
"#,
    )
    .unwrap();
    insta::assert_debug_snapshot!(m);
}
