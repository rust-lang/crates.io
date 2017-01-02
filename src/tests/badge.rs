use conduit::{Request, Method};
use postgres::GenericConnection;

use cargo_registry::db::RequestTransaction;
use cargo_registry::badge::Badge;

use std::collections::HashMap;

fn tx(req: &Request) -> &GenericConnection { req.tx().unwrap() }

#[test]
fn update_crate() {
    let (_b, app, _middle) = ::app();
    let mut req = ::req(app, Method::Get, "/api/v1/crates/badged_crate");

    ::mock_user(&mut req, ::user("foo"));
    let (krate, _) = ::mock_crate(&mut req, ::krate("badged_crate"));

    let appveyor = Badge::Appveyor {
        service: Some(String::from("github")),
        branch: None,
        repository: String::from("rust-lang/cargo"),
    };
    let mut badge_attributes_appveyor = HashMap::new();
    badge_attributes_appveyor.insert(
        String::from("service"),
        String::from("github")
    );
    badge_attributes_appveyor.insert(
        String::from("repository"),
        String::from("rust-lang/cargo")
    );

    let travis_ci = Badge::TravisCi {
        branch: Some(String::from("beta")),
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_travis_ci = HashMap::new();
    badge_attributes_travis_ci.insert(
        String::from("branch"),
        String::from("beta")
    );
    badge_attributes_travis_ci.insert(
        String::from("repository"),
        String::from("rust-lang/rust")
    );

    let mut badges = HashMap::new();

    // Updating with no badges has no effect
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![]);

    // Happy path adding one badge
    badges.insert(
        String::from("appveyor"),
        badge_attributes_appveyor.clone()
    );
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![appveyor.clone()]);

    // Replacing one badge with another
    badges.clear();
    badges.insert(
        String::from("travis-ci"),
        badge_attributes_travis_ci.clone()
    );
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![travis_ci.clone()]);

    // Updating badge attributes
    let travis_ci2 = Badge::TravisCi {
        branch: None,
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_travis_ci2 = HashMap::new();
    badge_attributes_travis_ci2.insert(
        String::from("repository"),
        String::from("rust-lang/rust")
    );
    badges.insert(
        String::from("travis-ci"),
        badge_attributes_travis_ci2.clone()
    );
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![travis_ci2.clone()]);

    // Removing one badge
    badges.clear();
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![]);

    // Adding 2 badges
    badges.insert(
        String::from("appveyor"),
        badge_attributes_appveyor.clone()
    );
    badges.insert(
        String::from("travis-ci"),
        badge_attributes_travis_ci.clone()
    );
    Badge::update_crate(
        tx(&req), &krate, badges.clone()
    ).unwrap();

    let current_badges = krate.badges(tx(&req)).unwrap();
    assert_eq!(current_badges.len(), 2);
    assert!(current_badges.contains(&appveyor));
    assert!(current_badges.contains(&travis_ci));

    // Removing all badges
    badges.clear();
    Badge::update_crate(tx(&req), &krate, badges.clone()).unwrap();
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![]);

    // Attempting to add one valid badge (appveyor) and two invalid badges
    // (travis-ci without a required attribute and an unknown badge type)

    // Extra invalid keys are fine, we'll just ignore those
    badge_attributes_appveyor.insert(
        String::from("extra"),
        String::from("info")
    );
    badges.insert(
        String::from("appveyor"),
        badge_attributes_appveyor.clone()
    );

    // Repository is a required key
    badge_attributes_travis_ci.remove("repository");
    badges.insert(
        String::from("travis-ci"),
        badge_attributes_travis_ci.clone()
    );

    // This is not a badge that crates.io knows about
    let mut invalid_attributes = HashMap::new();
    invalid_attributes.insert(
        String::from("not-a-badge-attribute"),
        String::from("not-a-badge-value")
    );
    badges.insert(
        String::from("not-a-badge"),
        invalid_attributes.clone()
    );

    let invalid_badges = Badge::update_crate(
        tx(&req), &krate, badges.clone()
    ).unwrap();
    assert_eq!(invalid_badges.len(), 2);
    assert!(invalid_badges.contains(&String::from("travis-ci")));
    assert!(invalid_badges.contains(&String::from("not-a-badge")));
    assert_eq!(krate.badges(tx(&req)).unwrap(), vec![appveyor.clone()]);
}
