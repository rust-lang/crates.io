use crates_io_cargo_toml::{
    Manifest,
    target_metadata::{Error, SourceFile, TargetMetadata},
};
use serde_json::json;
use std::str::FromStr;

fn extract(manifest: &str, files: &[&str]) -> Result<TargetMetadata, Error> {
    let mut manifest = Manifest::from_str(manifest).unwrap();
    TargetMetadata::extract(&mut manifest, files.iter().copied())
}

fn extract_error(manifest: &str) -> String {
    extract(manifest, &["Cargo.toml"]).unwrap_err().to_string()
}

#[test]
fn serializes_target_metadata() {
    let expected = json!({
        "build_script": { "path": "build.rs" },
        "library": null,
        "binaries": [],
    });
    let metadata: TargetMetadata = serde_json::from_value(expected.clone()).unwrap();
    let json = serde_json::to_value(&metadata).unwrap();

    assert!(metadata.build.as_ref().unwrap().exists);
    assert_eq!(json, expected);

    let missing = json!({
        "path": "custom/build.rs",
        "exists": false,
    });
    let missing: SourceFile = serde_json::from_value(missing).unwrap();
    assert!(!missing.exists);
}

#[test]
fn extracts_custom_and_inferred_targets() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"
        edition = "2021"
        build = "./tools/../tools/build.rs"

        [lib]
        name = "custom_library"
        path = "./source/lib.rs"
        crate-type = ["proc-macro"]

        [[bin]]
        name = "custom-binary"
        path = "commands/custom.rs"
        "#;
    let files = [
        "Cargo.toml",
        "tools/build.rs",
        "source/lib.rs",
        "commands/custom.rs",
        "src/main.rs",
        "src/bin/helper.rs",
    ];

    let metadata = extract(manifest, &files).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": { "path": "tools/build.rs" },
        "library": {
            "path": "source/lib.rs",
            "name": "custom_library",
            "is_proc_macro": true,
        },
        "binaries": [
            { "path": "commands/custom.rs", "name": "custom-binary" },
            { "path": "src/bin/helper.rs", "name": "helper" },
            { "path": "src/main.rs", "name": "target-demo" },
        ],
    });

    assert_eq!(json, expected);
}

#[test]
fn explicit_binary_disables_automatic_discovery_for_edition_2015() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"

        [[bin]]
        name = "custom-binary"
        path = "commands/custom.rs"
        "#;
    let files = [
        "Cargo.toml",
        "commands/custom.rs",
        "src/main.rs",
        "src/bin/helper.rs",
    ];

    let metadata = extract(manifest, &files).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": null,
        "library": null,
        "binaries": [{
            "path": "commands/custom.rs",
            "name": "custom-binary",
        }],
    });

    assert_eq!(json, expected);
}

#[test]
fn infers_default_targets() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"

        [lib]
        proc-macro = true
        "#;
    let files = ["Cargo.toml", "build.rs", "src/lib.rs", "src/main.rs"];

    let metadata = extract(manifest, &files).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": { "path": "build.rs" },
        "library": {
            "path": "src/lib.rs",
            "name": "target_demo",
            "is_proc_macro": true,
        },
        "binaries": [{
            "path": "src/main.rs",
            "name": "target-demo",
        }],
    });

    assert_eq!(json, expected);
}

#[test]
fn preserves_missing_declared_targets() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"
        build = "tools/build.rs"

        [lib]
        path = "source/lib.rs"

        [[bin]]
        name = "tool"
        path = "tools/tool.rs"
        "#;

    let metadata = extract(manifest, &["Cargo.toml"]).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": { "path": "tools/build.rs", "exists": false },
        "library": {
            "path": "source/lib.rs",
            "exists": false,
            "name": "target_demo",
            "is_proc_macro": false,
        },
        "binaries": [{
            "path": "tools/tool.rs",
            "exists": false,
            "name": "tool",
        }],
    });

    assert_eq!(json, expected);
}

#[test]
fn respects_disabled_build_scripts() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"
        build = false
        "#;

    let metadata = extract(manifest, &["Cargo.toml", "build.rs"]).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": null,
        "library": null,
        "binaries": [],
    });

    assert_eq!(json, expected);
}

#[test]
fn preserves_missing_default_build_script() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"
        build = true
        "#;

    let metadata = extract(manifest, &["Cargo.toml"]).unwrap();
    let build_script = metadata.build.unwrap();
    let expected = SourceFile {
        path: "build.rs".into(),
        exists: false,
    };

    assert_eq!(build_script, expected);
}

#[test]
fn rejects_paths_outside_the_package() {
    let errors = ["../tool.rs", "/tool.rs"].map(|path| {
        let manifest = format!(
            r#"
            [package]
            name = "target-demo"
            version = "1.0.0"

            [[bin]]
            name = "tool"
            path = "{path}"
            "#,
        );
        extract_error(&manifest)
    });

    insta::assert_snapshot!(errors.join("\n"), @r###"
    target path `../tool.rs` is outside the package root
    target path `/tool.rs` is outside the package root
    "###);
}

#[test]
fn rejects_empty_target_paths() {
    let errors = ["", ".", "tools/.."].map(|path| {
        let manifest = format!(
            r#"
            [package]
            name = "target-demo"
            version = "1.0.0"
            build = "{path}"
            "#
        );
        extract_error(&manifest)
    });

    insta::assert_snapshot!(errors.join("\n"), @r###"
    target path `` resolves to the package root
    target path `.` resolves to the package root
    target path `tools/..` resolves to the package root
    "###);
}

#[test]
fn normalizes_inventory_paths_for_directory_style_binaries() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"
        "#;
    let files = ["Cargo.toml", "./src/bin/other/../helper/main.rs"];

    let metadata = extract(manifest, &files).unwrap();
    let json = serde_json::to_value(metadata).unwrap();
    let expected = json!({
        "build_script": null,
        "library": null,
        "binaries": [{
            "path": "src/bin/helper/main.rs",
            "name": "helper",
        }],
    });

    assert_eq!(json, expected);
}

#[test]
fn rejects_unresolved_targets() {
    let manifest = r#"
        [package]
        name = "target-demo"
        version = "1.0.0"

        [[bin]]
        name = "tool"
        "#;

    let error = extract_error(manifest);

    insta::assert_snapshot!(error, @"binary `tool` target is missing its `path` field");
}
