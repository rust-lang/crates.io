use crate::builders::PublishBuilder;
use crate::util::{RequestHelper, TestApp};

#[test]
fn version_size() {
    let (_, _, user) = TestApp::full().with_user();

    let crate_to_publish = PublishBuilder::new("foo_version_size").version("1.0.0");
    user.publish_crate(crate_to_publish).good();

    // Add a file to version 2 so that it's a different size than version 1
    let files = [("foo_version_size-2.0.0/big", &[b'a'; 1] as &[_])];
    let crate_to_publish = PublishBuilder::new("foo_version_size")
        .version("2.0.0")
        .files(&files);
    user.publish_crate(crate_to_publish).good();

    let crate_json = user.show_crate("foo_version_size");

    let version1 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "1.0.0")
        .expect("Could not find v1.0.0");
    assert_eq!(version1.crate_size, Some(35));

    let version2 = crate_json
        .versions
        .as_ref()
        .unwrap()
        .iter()
        .find(|v| v.num == "2.0.0")
        .expect("Could not find v2.0.0");
    assert_eq!(version2.crate_size, Some(91));
}
