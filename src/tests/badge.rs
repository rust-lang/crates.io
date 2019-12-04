use crate::{builders::CrateBuilder, TestApp};
use cargo_registry::models::{Badge, Crate, MaintenanceStatus};
use std::collections::HashMap;

struct BadgeRef {
    appveyor: Badge,
    appveyor_attributes: HashMap<String, String>,
    travis_ci: Badge,
    travis_ci_attributes: HashMap<String, String>,
    gitlab: Badge,
    gitlab_attributes: HashMap<String, String>,
    azure_devops: Badge,
    azure_devops_attributes: HashMap<String, String>,
    isitmaintained_issue_resolution: Badge,
    isitmaintained_issue_resolution_attributes: HashMap<String, String>,
    isitmaintained_open_issues: Badge,
    isitmaintained_open_issues_attributes: HashMap<String, String>,
    codecov: Badge,
    codecov_attributes: HashMap<String, String>,
    coveralls: Badge,
    coveralls_attributes: HashMap<String, String>,
    circle_ci: Badge,
    circle_ci_attributes: HashMap<String, String>,
    cirrus_ci: Badge,
    cirrus_ci_attributes: HashMap<String, String>,
    bitbucket_pipelines: Badge,
    bitbucket_pipelines_attributes: HashMap<String, String>,
    maintenance: Badge,
    maintenance_attributes: HashMap<String, String>,
}

struct BadgeTestCrate {
    app: TestApp,
    krate: Crate,
}

impl BadgeTestCrate {
    /// Update the crate with badges, returning invalid badges
    fn update(&self, badges: &HashMap<String, HashMap<String, String>>) -> Vec<String> {
        self.app
            .db(|conn| Badge::update_crate(conn, &self.krate, Some(badges)).unwrap())
    }

    /// Update the crate to have no badges
    fn update_with_none(&self) {
        self.app
            .db(|conn| Badge::update_crate(conn, &self.krate, None).unwrap());
    }

    /// Return the crate's badges
    fn badges(&self) -> Vec<Badge> {
        self.app.db(|conn| self.krate.badges(conn).unwrap())
    }
}

fn set_up() -> (BadgeTestCrate, BadgeRef) {
    let (app, _, user) = TestApp::init().with_user();
    let user = user.as_model();

    let krate = app.db(|conn| CrateBuilder::new("badged_crate", user.id).expect_build(conn));

    let appveyor = Badge::Appveyor {
        service: Some(String::from("github")),
        id: None,
        branch: None,
        project_name: None,
        repository: String::from("rust-lang/cargo"),
    };
    let mut badge_attributes_appveyor = HashMap::new();
    badge_attributes_appveyor.insert(String::from("service"), String::from("github"));
    badge_attributes_appveyor.insert(String::from("repository"), String::from("rust-lang/cargo"));

    let travis_ci = Badge::TravisCi {
        branch: Some(String::from("beta")),
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_travis_ci = HashMap::new();
    badge_attributes_travis_ci.insert(String::from("branch"), String::from("beta"));
    badge_attributes_travis_ci.insert(String::from("repository"), String::from("rust-lang/rust"));

    let gitlab = Badge::GitLab {
        branch: Some(String::from("beta")),
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_gitlab = HashMap::new();
    badge_attributes_gitlab.insert(String::from("branch"), String::from("beta"));
    badge_attributes_gitlab.insert(String::from("repository"), String::from("rust-lang/rust"));

    let azure_devops = Badge::AzureDevops {
        project: String::from("rust-lang"),
        pipeline: String::from("rust"),
        build: Some(String::from("2")),
    };
    let mut badge_attributes_azure_devops = HashMap::new();
    badge_attributes_azure_devops.insert(String::from("project"), String::from("rust-lang"));
    badge_attributes_azure_devops.insert(String::from("pipeline"), String::from("rust"));
    badge_attributes_azure_devops.insert(String::from("build"), String::from("2"));

    let isitmaintained_issue_resolution = Badge::IsItMaintainedIssueResolution {
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_isitmaintained_issue_resolution = HashMap::new();
    badge_attributes_isitmaintained_issue_resolution
        .insert(String::from("repository"), String::from("rust-lang/rust"));

    let isitmaintained_open_issues = Badge::IsItMaintainedOpenIssues {
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_isitmaintained_open_issues = HashMap::new();
    badge_attributes_isitmaintained_open_issues
        .insert(String::from("repository"), String::from("rust-lang/rust"));

    let codecov = Badge::Codecov {
        service: Some(String::from("github")),
        branch: Some(String::from("beta")),
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_codecov = HashMap::new();
    badge_attributes_codecov.insert(String::from("branch"), String::from("beta"));
    badge_attributes_codecov.insert(String::from("repository"), String::from("rust-lang/rust"));
    badge_attributes_codecov.insert(String::from("service"), String::from("github"));

    let coveralls = Badge::Coveralls {
        service: Some(String::from("github")),
        branch: Some(String::from("beta")),
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_coveralls = HashMap::new();
    badge_attributes_coveralls.insert(String::from("branch"), String::from("beta"));
    badge_attributes_coveralls.insert(String::from("repository"), String::from("rust-lang/rust"));
    badge_attributes_coveralls.insert(String::from("service"), String::from("github"));

    let circle_ci = Badge::CircleCi {
        repository: String::from("rust-lang/rust"),
        branch: Some(String::from("beta")),
    };
    let mut badge_attributes_circle_ci = HashMap::new();
    badge_attributes_circle_ci.insert(String::from("branch"), String::from("beta"));
    badge_attributes_circle_ci.insert(String::from("repository"), String::from("rust-lang/rust"));

    let cirrus_ci = Badge::CirrusCi {
        repository: String::from("rust-lang/rust"),
        branch: Some(String::from("beta")),
    };
    let mut badge_attributes_cirrus_ci = HashMap::new();
    badge_attributes_cirrus_ci.insert(String::from("branch"), String::from("beta"));
    badge_attributes_cirrus_ci.insert(String::from("repository"), String::from("rust-lang/rust"));

    let bitbucket_pipelines = Badge::BitbucketPipelines {
        repository: String::from("rust-lang/rust"),
        branch: String::from("beta"),
    };
    let mut badge_attributes_bitbucket_pipelines = HashMap::new();
    badge_attributes_bitbucket_pipelines
        .insert(String::from("repository"), String::from("rust-lang/rust"));
    badge_attributes_bitbucket_pipelines.insert(String::from("branch"), String::from("beta"));

    let maintenance = Badge::Maintenance {
        status: MaintenanceStatus::LookingForMaintainer,
    };
    let mut maintenance_attributes = HashMap::new();
    maintenance_attributes.insert(
        String::from("status"),
        String::from("looking-for-maintainer"),
    );

    let badges = BadgeRef {
        appveyor,
        appveyor_attributes: badge_attributes_appveyor,
        travis_ci,
        travis_ci_attributes: badge_attributes_travis_ci,
        gitlab,
        gitlab_attributes: badge_attributes_gitlab,
        azure_devops,
        azure_devops_attributes: badge_attributes_azure_devops,
        isitmaintained_issue_resolution,
        isitmaintained_issue_resolution_attributes:
            badge_attributes_isitmaintained_issue_resolution,
        isitmaintained_open_issues,
        isitmaintained_open_issues_attributes: badge_attributes_isitmaintained_open_issues,
        codecov,
        codecov_attributes: badge_attributes_codecov,
        coveralls,
        coveralls_attributes: badge_attributes_coveralls,
        circle_ci,
        circle_ci_attributes: badge_attributes_circle_ci,
        cirrus_ci,
        cirrus_ci_attributes: badge_attributes_cirrus_ci,
        bitbucket_pipelines,
        bitbucket_pipelines_attributes: badge_attributes_bitbucket_pipelines,
        maintenance,
        maintenance_attributes,
    };
    (BadgeTestCrate { app, krate }, badges)
}

#[test]
fn update_no_badges() {
    // Add no badges
    let (krate, _) = set_up();

    // Updating with no badges has no effect
    krate.update_with_none();
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn update_add_appveyor() {
    // Add an appveyor badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("appveyor"), test_badges.appveyor_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.appveyor]);
}

#[test]
fn update_add_travis_ci() {
    // Add a travis ci badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("travis-ci"), test_badges.travis_ci_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.travis_ci]);
}

#[test]
fn update_add_gitlab() {
    // Add a gitlab badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("gitlab"), test_badges.gitlab_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.gitlab]);
}

#[test]
fn update_add_azure_devops() {
    // Add a azure devops badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(
        String::from("azure-devops"),
        test_badges.azure_devops_attributes,
    );
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.azure_devops]);
}

#[test]
fn update_add_isitmaintained_issue_resolution() {
    // Add a isitmaintained_issue_resolution badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(
        String::from("is-it-maintained-issue-resolution"),
        test_badges.isitmaintained_issue_resolution_attributes,
    );
    krate.update(&badges);
    assert_eq!(
        krate.badges(),
        vec![test_badges.isitmaintained_issue_resolution]
    );
}

#[test]
fn update_add_isitmaintained_open_issues() {
    // Add a isitmaintained_open_issues badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(
        String::from("is-it-maintained-open-issues"),
        test_badges.isitmaintained_open_issues_attributes,
    );
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.isitmaintained_open_issues]);
}

#[test]
fn update_add_codecov() {
    // Add a codecov badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("codecov"), test_badges.codecov_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.codecov]);
}

#[test]
fn update_add_coveralls() {
    // Add a coveralls badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("coveralls"), test_badges.coveralls_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.coveralls]);
}

#[test]
fn update_add_circle_ci() {
    // Add a CircleCI badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("circle-ci"), test_badges.circle_ci_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.circle_ci]);
}

#[test]
fn update_add_cirrus_ci() {
    // Add a Cirrus CI badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(String::from("cirrus-ci"), test_badges.cirrus_ci_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.cirrus_ci]);
}

#[test]
fn update_add_bitbucket_pipelines() {
    // Add a Bitbucket Pipelines badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(
        String::from("bitbucket-pipelines"),
        test_badges.bitbucket_pipelines_attributes,
    );
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.bitbucket_pipelines]);
}

#[test]
fn update_add_maintenance() {
    // Add a maintenance badge
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();
    badges.insert(
        String::from("maintenance"),
        test_badges.maintenance_attributes,
    );
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.maintenance]);
}

#[test]
fn replace_badge() {
    // Replacing one badge with another
    let (krate, test_badges) = set_up();

    // Add a badge
    let mut badges = HashMap::new();
    badges.insert(String::from("gitlab"), test_badges.gitlab_attributes);
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.gitlab]);
    // Replace with another badge
    badges.clear();
    badges.insert(
        String::from("travis-ci"),
        test_badges.travis_ci_attributes.clone(),
    );
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.travis_ci]);
}

#[test]
fn update_attributes() {
    // Update badge attributes
    let (krate, test_badges) = set_up();

    // Add a travis-ci badge
    let mut badges = HashMap::new();
    badges.insert(String::from("travis-ci"), test_badges.travis_ci_attributes);
    krate.update(&badges);
    let current_badges = krate.badges();
    assert_eq!(current_badges.len(), 1);
    assert!(current_badges.contains(&test_badges.travis_ci));

    // Now update the travis ci badge with different attributes
    let mut badges = HashMap::new();
    let travis_ci2 = Badge::TravisCi {
        branch: None,
        repository: String::from("rust-lang/rust"),
    };
    let mut badge_attributes_travis_ci2 = HashMap::new();
    badge_attributes_travis_ci2.insert(String::from("repository"), String::from("rust-lang/rust"));
    badges.insert(String::from("travis-ci"), badge_attributes_travis_ci2);
    krate.update(&badges);
    let current_badges = krate.badges();
    assert_eq!(current_badges.len(), 1);
    assert!(current_badges.contains(&travis_ci2));
}

#[test]
fn clear_badges() {
    // Add 3 badges and then remove them
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();

    // Adding 3 badges
    badges.insert(String::from("appveyor"), test_badges.appveyor_attributes);
    badges.insert(String::from("travis-ci"), test_badges.travis_ci_attributes);
    badges.insert(String::from("gitlab"), test_badges.gitlab_attributes);
    krate.update(&badges);
    let current_badges = krate.badges();
    assert_eq!(current_badges.len(), 3);
    assert!(current_badges.contains(&test_badges.appveyor));
    assert!(current_badges.contains(&test_badges.travis_ci));
    assert!(current_badges.contains(&test_badges.gitlab));

    // Removing all badges
    badges.clear();
    krate.update(&badges);
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn appveyor_extra_keys() {
    // Add a badge with extra invalid keys
    let (krate, test_badges) = set_up();

    let mut badges = HashMap::new();

    // Extra invalid keys are fine, they just get ignored
    let mut appveyor_attributes = test_badges.appveyor_attributes.clone();
    appveyor_attributes.insert(String::from("extra"), String::from("info"));
    badges.insert(String::from("appveyor"), test_badges.appveyor_attributes);

    krate.update(&badges);
    assert_eq!(krate.badges(), vec![test_badges.appveyor]);
}

#[test]
fn travis_ci_required_keys() {
    // Add a travis ci badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.travis_ci_attributes.remove("repository");
    badges.insert(String::from("travis-ci"), test_badges.travis_ci_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "travis-ci");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn gitlab_required_keys() {
    // Add a gitlab badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.gitlab_attributes.remove("repository");
    badges.insert(String::from("gitlab"), test_badges.gitlab_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "gitlab");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn azure_devops_required_keys() {
    // Add a azure devops badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // project is a required key
    test_badges.azure_devops_attributes.remove("project");
    badges.insert(
        String::from("azure-devops"),
        test_badges.azure_devops_attributes,
    );

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "azure-devops");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn isitmaintained_issue_resolution_required_keys() {
    // Add a isitmaintained_issue_resolution badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges
        .isitmaintained_issue_resolution_attributes
        .remove("repository");
    badges.insert(
        String::from("isitmaintained_issue_resolution"),
        test_badges.isitmaintained_issue_resolution_attributes,
    );

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(
        invalid_badges.first().unwrap(),
        "isitmaintained_issue_resolution"
    );
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn isitmaintained_open_issues_required_keys() {
    // Add a isitmaintained_open_issues badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges
        .isitmaintained_open_issues_attributes
        .remove("repository");
    badges.insert(
        String::from("isitmaintained_open_issues"),
        test_badges.isitmaintained_open_issues_attributes,
    );

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(
        invalid_badges.first().unwrap(),
        "isitmaintained_open_issues"
    );
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn codecov_required_keys() {
    // Add a codecov badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.codecov_attributes.remove("repository");
    badges.insert(String::from("codecov"), test_badges.codecov_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "codecov");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn coveralls_required_keys() {
    // Add a coveralls badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.coveralls_attributes.remove("repository");
    badges.insert(String::from("coveralls"), test_badges.coveralls_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "coveralls");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn circle_ci_required_keys() {
    // Add a CircleCI badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.circle_ci_attributes.remove("repository");
    badges.insert(String::from("circle-ci"), test_badges.circle_ci_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "circle-ci");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn cirrus_ci_required_keys() {
    // Add a Cirrus CI badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Repository is a required key
    test_badges.cirrus_ci_attributes.remove("repository");
    badges.insert(String::from("cirrus-ci"), test_badges.cirrus_ci_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "cirrus-ci");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn bitbucket_pipelines_required_keys() {
    // Add a Bitbucket Pipelines badge missing a required field
    let (krate, test_badges) = set_up();

    for required in &["repository", "branch"] {
        let mut attributes = test_badges.bitbucket_pipelines_attributes.clone();
        attributes.remove(*required);

        let mut badges = HashMap::new();
        badges.insert(String::from("bitbucket-pipelines"), attributes);

        let invalid_badges = krate.update(&badges);
        assert_eq!(invalid_badges.len(), 1);
        assert_eq!(invalid_badges.first().unwrap(), "bitbucket-pipelines");
        assert_eq!(krate.badges(), vec![]);
    }
}

#[test]
fn maintenance_required_keys() {
    // Add a maintenance badge missing a required field
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // Status is a required key
    test_badges.maintenance_attributes.remove("status");
    badges.insert(
        String::from("maintenance"),
        test_badges.maintenance_attributes,
    );

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "maintenance");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn maintenance_invalid_values() {
    // Add a maintenance badge with an invalid value
    let (krate, mut test_badges) = set_up();

    let mut badges = HashMap::new();

    // "totes broken" is not a recognized value
    test_badges
        .maintenance_attributes
        .insert(String::from("status"), String::from("totes broken"));
    badges.insert(
        String::from("maintenance"),
        test_badges.maintenance_attributes,
    );

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "maintenance");
    assert_eq!(krate.badges(), vec![]);
}

#[test]
fn unknown_badge() {
    // Add an unknown badge
    let (krate, _) = set_up();

    let mut badges = HashMap::new();

    // This is not a badge that crates.io knows about
    let mut invalid_attributes = HashMap::new();
    invalid_attributes.insert(
        String::from("not-a-badge-attribute"),
        String::from("not-a-badge-value"),
    );
    badges.insert(String::from("not-a-badge"), invalid_attributes);

    let invalid_badges = krate.update(&badges);
    assert_eq!(invalid_badges.len(), 1);
    assert_eq!(invalid_badges.first().unwrap(), "not-a-badge");
    assert_eq!(krate.badges(), vec![]);
}
