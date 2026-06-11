use cargo_manifest::{Manifest, Product};

mod utils;

const BASIC_MANIFEST: &str = r#"
[package]
name = "test-package"
version = "0.1.0"
"#;

fn full_example_extra_files() -> Vec<&'static str> {
    vec![
        "benches/large-input.rs",
        "benches/multi-file-bench/bench_module.rs",
        "benches/multi-file-bench/main.rs",
        "examples/multi-file-example/ex_module.rs",
        "examples/multi-file-example/main.rs",
        "examples/simple.rs",
        "src/bin/another-executable.rs",
        "src/bin/multi-file-executable/main.rs",
        "src/bin/multi-file-executable/some_module.rs",
        "src/bin/named-executable.rs",
        "src/lib.rs",
        "src/main.rs",
        "tests/multi-file-test/main.rs",
        "tests/multi-file-test/test_module.rs",
        "tests/some-integration-tests.rs",
    ]
}

fn format_products(products: &[Product]) -> String {
    products
        .iter()
        .map(format_product)
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_product(product: &Product) -> String {
    let name = product.name.as_deref().unwrap_or("<None>");
    let path = product.path.as_deref().unwrap_or("<None>");
    format!("{name}  →  {path}")
}

#[test]
fn test_full_example() {
    let tempdir = utils::prepare(BASIC_MANIFEST, full_example_extra_files());
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
    insta::assert_snapshot!(format_product(&m.lib.unwrap()), @"test_package  →  src/lib.rs");

    insta::assert_snapshot!(format_products(&m.bin), @r###"
    another-executable  →  src/bin/another-executable.rs
    multi-file-executable  →  src/bin/multi-file-executable/main.rs
    named-executable  →  src/bin/named-executable.rs
    test-package  →  src/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.example), @r###"
    multi-file-example  →  examples/multi-file-example/main.rs
    simple  →  examples/simple.rs
    "###);

    insta::assert_snapshot!(format_products(&m.test), @r###"
    multi-file-test  →  tests/multi-file-test/main.rs
    some-integration-tests  →  tests/some-integration-tests.rs
    "###);

    insta::assert_snapshot!(format_products(&m.bench), @r###"
    large-input  →  benches/large-input.rs
    multi-file-bench  →  benches/multi-file-bench/main.rs
    "###);
}

#[test]
fn test_full_example_with_declarations_2024() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"
    edition = "2024"

    [[bin]]
    name = "named-executable"

    [[example]]
    name = "simple"

    [[test]]
    name = "some-integration-tests"

    [[bench]]
    name = "large-input"
    "#;
    let tempdir = utils::prepare(manifest, full_example_extra_files());
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
    insta::assert_snapshot!(format_product(&m.lib.unwrap()), @"test_package  →  src/lib.rs");

    insta::assert_snapshot!(format_products(&m.bin), @r###"
    named-executable  →  src/bin/named-executable.rs
    another-executable  →  src/bin/another-executable.rs
    multi-file-executable  →  src/bin/multi-file-executable/main.rs
    test-package  →  src/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.example), @r###"
    simple  →  examples/simple.rs
    multi-file-example  →  examples/multi-file-example/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.test), @r###"
    some-integration-tests  →  tests/some-integration-tests.rs
    multi-file-test  →  tests/multi-file-test/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.bench), @r###"
    large-input  →  benches/large-input.rs
    multi-file-bench  →  benches/multi-file-bench/main.rs
    "###);
}

#[test]
fn test_full_example_with_declarations_2021() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"
    edition = "2021"

    [[bin]]
    name = "named-executable"

    [[example]]
    name = "simple"

    [[test]]
    name = "some-integration-tests"

    [[bench]]
    name = "large-input"
    "#;
    let tempdir = utils::prepare(manifest, full_example_extra_files());
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
    insta::assert_snapshot!(format_product(&m.lib.unwrap()), @"test_package  →  src/lib.rs");

    insta::assert_snapshot!(format_products(&m.bin), @r###"
    named-executable  →  src/bin/named-executable.rs
    another-executable  →  src/bin/another-executable.rs
    multi-file-executable  →  src/bin/multi-file-executable/main.rs
    test-package  →  src/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.example), @r###"
    simple  →  examples/simple.rs
    multi-file-example  →  examples/multi-file-example/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.test), @r###"
    some-integration-tests  →  tests/some-integration-tests.rs
    multi-file-test  →  tests/multi-file-test/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.bench), @r###"
    large-input  →  benches/large-input.rs
    multi-file-bench  →  benches/multi-file-bench/main.rs
    "###);
}

#[test]
fn test_full_example_with_declarations_2015() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"

    [[bin]]
    name = "named-executable"

    [[example]]
    name = "simple"

    [[test]]
    name = "some-integration-tests"

    [[bench]]
    name = "large-input"
    "#;
    let tempdir = utils::prepare(manifest, full_example_extra_files());
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
    insta::assert_snapshot!(format_product(&m.lib.unwrap()), @"test_package  →  src/lib.rs");
    insta::assert_snapshot!(format_products(&m.bin), @"named-executable  →  src/bin/named-executable.rs");
    insta::assert_snapshot!(format_products(&m.example), @"simple  →  examples/simple.rs");
    insta::assert_snapshot!(format_products(&m.test), @"some-integration-tests  →  tests/some-integration-tests.rs");
    insta::assert_snapshot!(format_products(&m.bench), @"large-input  →  benches/large-input.rs");
}

#[test]
fn test_full_example_without_discovery() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"
    edition = "2021"
    autobins = false
    autoexamples = false
    autotests = false
    autobenches = false
    "#;
    let tempdir = utils::prepare(manifest, full_example_extra_files());
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_debug_snapshot!(m);
    insta::assert_snapshot!(format_products(&m.bin), @"");
    insta::assert_snapshot!(format_products(&m.example), @"");
    insta::assert_snapshot!(format_products(&m.test), @"");
    insta::assert_snapshot!(format_products(&m.bench), @"");
}

/// Check that broken paths are handled without errors. It is up to the
/// user to potentially turn this into a warning or error.
#[test]
fn test_declarations_with_broken_paths() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"

    [[bin]]
    name = "named-executable"
    path = "named-executable.rs"

    [[example]]
    name = "simple"
    path = "simple.rs"

    [[test]]
    name = "some-integration-tests"
    path = "some-integration-tests.rs"

    [[bench]]
    name = "large-input"
    path = "large-input.rs"
    "#;
    let tempdir = utils::prepare(manifest, vec![]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_snapshot!(format_products(&m.bin), @"named-executable  →  named-executable.rs");
    insta::assert_snapshot!(format_products(&m.example), @"simple  →  simple.rs");
    insta::assert_snapshot!(format_products(&m.test), @"some-integration-tests  →  some-integration-tests.rs");
    insta::assert_snapshot!(format_products(&m.bench), @"large-input  →  large-input.rs");
}

/// Check that missing and broken paths are handled without errors. It is up
/// to the user to potentially turn this into a warning or error.
#[test]
fn test_declarations_with_missing_and_broken_paths() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"

    [[bin]]
    name = "named-executable"

    [[example]]
    name = "simple"

    [[test]]
    name = "some-integration-tests"

    [[bench]]
    name = "large-input"
    "#;
    let tempdir = utils::prepare(manifest, vec![]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_snapshot!(format_products(&m.bin), @"named-executable  →  <None>");
    insta::assert_snapshot!(format_products(&m.example), @"simple  →  <None>");
    insta::assert_snapshot!(format_products(&m.test), @"some-integration-tests  →  <None>");
    insta::assert_snapshot!(format_products(&m.bench), @"large-input  →  <None>");
}

/// Check that duplicate names are handled without errors. It is up to the
/// user to potentially turn this into a warning or error.
#[test]
fn test_duplicate_names() {
    let tempdir = utils::prepare(
        BASIC_MANIFEST,
        vec![
            "benches/large-input.rs",
            "benches/large-input/main.rs",
            "examples/simple.rs",
            "examples/simple/main.rs",
            "src/bin/test-package.rs",
            "src/bin/test-package/main.rs",
            "src/main.rs",
            "tests/some-tests.rs",
            "tests/some-tests/main.rs",
        ],
    );
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();

    insta::assert_snapshot!(format_products(&m.bin), @r###"
    test-package  →  src/bin/test-package/main.rs
    test-package  →  src/bin/test-package.rs
    test-package  →  src/main.rs
    "###);

    insta::assert_snapshot!(format_products(&m.example), @r###"
    simple  →  examples/simple/main.rs
    simple  →  examples/simple.rs
    "###);

    insta::assert_snapshot!(format_products(&m.test), @r###"
    some-tests  →  tests/some-tests/main.rs
    some-tests  →  tests/some-tests.rs
    "###);

    insta::assert_snapshot!(format_products(&m.bench), @r###"
    large-input  →  benches/large-input/main.rs
    large-input  →  benches/large-input.rs
    "###);
}

/// Check that missing names are handled without errors. It is up to the
/// user to potentially turn this into a warning or error.
#[test]
fn test_missing_names() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"

    [[bin]]
    path = "named-executable.rs"

    [[example]]
    path = "simple.rs"

    [[test]]
    path = "some-integration-tests.rs"

    [[bench]]
    path = "large-input.rs"
    "#;
    let tempdir = utils::prepare(manifest, vec![]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_snapshot!(format_products(&m.bin), @"<None>  →  named-executable.rs");
    insta::assert_snapshot!(format_products(&m.example), @"<None>  →  simple.rs");
    insta::assert_snapshot!(format_products(&m.test), @"<None>  →  some-integration-tests.rs");
    insta::assert_snapshot!(format_products(&m.bench), @"<None>  →  large-input.rs");
}

/// see https://doc.rust-lang.org/cargo/reference/cargo-targets.html#target-auto-discovery
#[test]
fn test_bin_module_example() {
    let manifest = r#"
    [package]
    name = "test-package"
    version = "0.1.0"
    autobins = false
    "#;
    let tempdir = utils::prepare(manifest, vec!["src/lib.rs", "src/bin/mod.rs"]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    insta::assert_snapshot!(format_product(&m.lib.unwrap()), @"test_package  →  src/lib.rs");
    insta::assert_snapshot!(format_products(&m.bin), @"");
    insta::assert_snapshot!(format_products(&m.example), @"");
    insta::assert_snapshot!(format_products(&m.test), @"");
    insta::assert_snapshot!(format_products(&m.bench), @"");
}

/// see <https://github.com/rust-lang/crates.io/issues/9222>
#[test]
fn test_data_encoding_bin() {
    let manifest = r#"
    [package]
    name = "data-encoding-bin"
    version = "0.3.4"
    license = "MIT"
    edition = "2021"
    keywords = ["base-conversion", "encoding", "base64", "base32", "hex"]
    categories = ["command-line-utilities", "encoding"]
    readme = "README.md"
    repository = "https://github.com/ia0/data-encoding"
    description = "Swiss Army knife for data-encoding"
    include = ["Cargo.toml", "LICENSE", "README.md", "src/main.rs"]

    [[bin]]
    name = "data-encoding"
    path = "src/main.rs"

    [dependencies]
    data-encoding = { version = "2.6.0", path = "../lib" }
    getopts = "0.2"
    "#;
    let tempdir = utils::prepare(manifest, vec!["src/main.rs"]);
    let m = Manifest::from_path(tempdir.path().join("Cargo.toml")).unwrap();
    assert!(m.lib.is_none());
    insta::assert_snapshot!(format_products(&m.bin), @"data-encoding  →  src/main.rs");
    insta::assert_snapshot!(format_products(&m.example), @"");
    insta::assert_snapshot!(format_products(&m.test), @"");
    insta::assert_snapshot!(format_products(&m.bench), @"");
}
