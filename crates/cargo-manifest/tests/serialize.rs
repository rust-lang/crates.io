use cargo_manifest::{Manifest, Package};

#[test]
fn basic() {
    let manifest: Manifest<(), ()> = Manifest {
        package: Some(Package::new("foo".into(), "1.0.0".into())),
        ..Default::default()
    };

    let serialized = toml::to_string(&manifest).unwrap();
    insta::assert_snapshot!(serialized);
}
