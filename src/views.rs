use chrono::{DateTime, Utc};

use crate::external_urls::remove_blocked_urls;
use crate::models::{
    ApiToken, Category, Crate, Dependency, DependencyKind, Keyword, Owner, ReverseDependency, Team,
    TopVersions, User, Version, VersionDownload, VersionOwnerAction,
};
use crates_io_github as github;

pub mod krate_publish;
pub use self::krate_publish::{EncodableCrateDependency, PublishMetadata};

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
#[schema(as = Category)]
pub struct EncodableCategory {
    /// An opaque identifier for the category.
    #[schema(example = "game-development")]
    pub id: String,

    /// The name of the category.
    #[schema(example = "Game development")]
    pub category: String,

    /// The "slug" of the category.
    ///
    /// See <https://crates.io/category_slugs>.
    #[schema(example = "game-development")]
    pub slug: String,

    /// A description of the category.
    #[schema(example = "Libraries for creating games.")]
    pub description: String,

    /// The date and time this category was created.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub created_at: DateTime<Utc>,

    /// The total number of crates that have this category.
    #[schema(example = 42)]
    pub crates_cnt: i32,

    /// The subcategories of this category.
    ///
    /// This field is only present when the category details are queried,
    /// but not when listing categories.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(no_recursion, example = json!([]))]
    pub subcategories: Option<Vec<EncodableCategory>>,

    /// The parent categories of this category.
    ///
    /// This field is only present when the category details are queried,
    /// but not when listing categories.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[schema(no_recursion, example = json!([]))]
    pub parent_categories: Option<Vec<EncodableCategory>>,
}

impl From<Category> for EncodableCategory {
    fn from(category: Category) -> Self {
        let Category {
            crates_cnt,
            category,
            slug,
            description,
            created_at,
            ..
        } = category;
        Self {
            id: slug.clone(),
            slug,
            description,
            created_at,
            crates_cnt,
            category: category.rsplit("::").collect::<Vec<_>>()[0].to_string(),
            subcategories: None,
            parent_categories: None,
        }
    }
}

/// The serialization format for the `CrateOwnerInvitation` model.
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct EncodableCrateOwnerInvitationV1 {
    pub invitee_id: i32,
    pub inviter_id: i32,
    pub invited_by_username: String,
    pub crate_name: String,
    pub crate_id: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct EncodableCrateOwnerInvitation {
    pub invitee_id: i32,
    pub inviter_id: i32,
    pub crate_id: i32,
    pub crate_name: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub struct InvitationResponse {
    pub crate_id: i32,
    pub accepted: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableDependency {
    pub id: i32,
    pub version_id: i32,
    pub crate_id: String,
    pub req: String,
    pub optional: bool,
    pub default_features: bool,
    pub features: Vec<String>,
    pub target: Option<String>,
    pub kind: DependencyKind,
    pub downloads: i64,
}

impl EncodableDependency {
    pub fn from_dep(dependency: Dependency, crate_name: &str) -> Self {
        Self::encode(dependency, crate_name, None)
    }

    pub fn from_reverse_dep(rev_dep: ReverseDependency, crate_name: &str) -> Self {
        let dependency = rev_dep.dependency;
        Self::encode(dependency, crate_name, Some(rev_dep.crate_downloads))
    }

    // `downloads` need only be specified when generating a reverse dependency
    fn encode(dependency: Dependency, crate_name: &str, downloads: Option<i64>) -> Self {
        Self {
            id: dependency.id,
            version_id: dependency.version_id,
            crate_id: crate_name.into(),
            req: dependency.req,
            optional: dependency.optional,
            default_features: dependency.default_features,
            features: dependency.features,
            target: dependency.target,
            kind: dependency.kind,
            downloads: downloads.unwrap_or(0),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableVersionDownload {
    pub version: i32,
    pub downloads: i32,
    pub date: String,
}

impl From<VersionDownload> for EncodableVersionDownload {
    fn from(download: VersionDownload) -> Self {
        Self {
            version: download.version_id,
            downloads: download.downloads,
            date: download.date.to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
#[schema(as = Keyword)]
pub struct EncodableKeyword {
    /// An opaque identifier for the keyword.
    #[schema(example = "http")]
    pub id: String,

    /// The keyword itself.
    #[schema(example = "http")]
    pub keyword: String,

    /// The date and time this keyword was created.
    #[schema(example = "2017-01-06T14:23:11Z")]
    pub created_at: DateTime<Utc>,

    /// The total number of crates that have this keyword.
    #[schema(example = 42)]
    pub crates_cnt: i32,
}

impl From<Keyword> for EncodableKeyword {
    fn from(keyword: Keyword) -> Self {
        let Keyword {
            crates_cnt,
            keyword,
            created_at,
            ..
        } = keyword;
        Self {
            id: keyword.clone(),
            created_at,
            crates_cnt,
            keyword,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrate {
    pub id: String,
    pub name: String,
    pub updated_at: DateTime<Utc>,
    pub versions: Option<Vec<i32>>,
    pub keywords: Option<Vec<String>>,
    pub categories: Option<Vec<String>>,
    pub badges: [(); 0],
    pub created_at: DateTime<Utc>,
    // NOTE: Used by shields.io, altering `downloads` requires a PR with shields.io
    pub downloads: i64,
    pub recent_downloads: Option<i64>,
    pub default_version: Option<String>,
    pub num_versions: i32,
    pub yanked: bool,
    // NOTE: Used by shields.io, altering `max_version` requires a PR with shields.io
    pub max_version: String,
    pub newest_version: String, // Most recently updated version, which may not be max
    pub max_stable_version: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
    pub links: EncodableCrateLinks,
    pub exact_match: bool,
}

impl EncodableCrate {
    #[allow(clippy::too_many_arguments)]
    pub fn from(
        krate: Crate,
        default_version: Option<&str>,
        num_versions: i32,
        yanked: Option<bool>,
        top_versions: Option<&TopVersions>,
        versions: Option<Vec<i32>>,
        keywords: Option<&[Keyword]>,
        categories: Option<&[Category]>,
        exact_match: bool,
        downloads: i64,
        recent_downloads: Option<i64>,
    ) -> Self {
        let Crate {
            name,
            created_at,
            updated_at,
            description,
            homepage,
            documentation,
            repository,
            ..
        } = krate;
        let versions_link = match versions {
            Some(..) => None,
            None => Some(format!("/api/v1/crates/{name}/versions")),
        };
        let keyword_ids = keywords.map(|kws| kws.iter().map(|kw| kw.keyword.clone()).collect());
        let category_ids = categories.map(|cats| cats.iter().map(|cat| cat.slug.clone()).collect());
        let homepage = remove_blocked_urls(homepage);
        let documentation = remove_blocked_urls(documentation);
        let repository = remove_blocked_urls(repository);

        let default_version = default_version.map(ToString::to_string);
        if default_version.is_none() {
            let message = format!("Crate `{name}` has no default version");
            sentry::capture_message(&message, sentry::Level::Info);
        }
        let yanked = yanked.unwrap_or_default();

        let max_version = top_versions
            .and_then(|v| v.highest.as_ref())
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0.0.0".to_string());

        let newest_version = top_versions
            .and_then(|v| v.newest.as_ref())
            .map(|v| v.to_string())
            .unwrap_or_else(|| "0.0.0".to_string());

        let max_stable_version = top_versions
            .and_then(|v| v.highest_stable.as_ref())
            .map(|v| v.to_string());

        // the total number of downloads is eventually consistent, but can lag
        // behind the number of "recent downloads". to hide this inconsistency
        // we will use the "recent downloads" as "total downloads" in case it is
        // higher.
        let downloads = if matches!(recent_downloads, Some(x) if x > downloads) {
            recent_downloads.unwrap()
        } else {
            downloads
        };

        EncodableCrate {
            id: name.clone(),
            name: name.clone(),
            updated_at,
            created_at,
            downloads,
            recent_downloads,
            versions,
            keywords: keyword_ids,
            categories: category_ids,
            badges: [],
            default_version,
            num_versions,
            yanked,
            max_version,
            newest_version,
            max_stable_version,
            documentation,
            homepage,
            exact_match,
            description,
            repository,
            links: EncodableCrateLinks {
                version_downloads: format!("/api/v1/crates/{name}/downloads"),
                versions: versions_link,
                owners: Some(format!("/api/v1/crates/{name}/owners")),
                owner_team: Some(format!("/api/v1/crates/{name}/owner_team")),
                owner_user: Some(format!("/api/v1/crates/{name}/owner_user")),
                reverse_dependencies: format!("/api/v1/crates/{name}/reverse_dependencies"),
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn from_minimal(
        krate: Crate,
        default_version: Option<&str>,
        num_versions: i32,
        yanked: Option<bool>,
        top_versions: Option<&TopVersions>,
        exact_match: bool,
        downloads: i64,
        recent_downloads: Option<i64>,
    ) -> Self {
        Self::from(
            krate,
            default_version,
            num_versions,
            yanked,
            top_versions,
            None,
            None,
            None,
            exact_match,
            downloads,
            recent_downloads,
        )
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableCrateLinks {
    pub version_downloads: String,
    pub versions: Option<String>,
    pub owners: Option<String>,
    pub owner_team: Option<String>,
    pub owner_user: Option<String>,
    pub reverse_dependencies: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableOwner {
    pub id: i32,
    pub login: String,
    pub kind: String,
    pub url: Option<String>,
    pub name: Option<String>,
    pub avatar: Option<String>,
}

impl From<Owner> for EncodableOwner {
    fn from(owner: Owner) -> Self {
        match owner {
            Owner::User(User {
                id,
                name,
                gh_login,
                gh_avatar,
                ..
            }) => {
                let url = format!("https://github.com/{gh_login}");
                Self {
                    id,
                    login: gh_login,
                    avatar: gh_avatar,
                    url: Some(url),
                    name,
                    kind: String::from("user"),
                }
            }
            Owner::Team(Team {
                id,
                name,
                login,
                avatar,
                ..
            }) => {
                let url = github::team_url(&login);
                Self {
                    id,
                    login,
                    url: Some(url),
                    avatar,
                    name,
                    kind: String::from("team"),
                }
            }
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EncodableTeam {
    pub id: i32,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
}

impl From<Team> for EncodableTeam {
    fn from(team: Team) -> Self {
        let Team {
            id,
            name,
            login,
            avatar,
            ..
        } = team;
        let url = github::team_url(&login);

        EncodableTeam {
            id,
            login,
            name,
            avatar,
            url: Some(url),
        }
    }
}

/// The serialization format for the `ApiToken` model with its token value.
/// This should only be used when initially creating a new token to minimize
/// the chance of token leaks.
#[derive(Serialize, Debug)]
pub struct EncodableApiTokenWithToken {
    #[serde(flatten)]
    pub token: ApiToken,
    #[serde(rename = "token")]
    pub plaintext: String,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct OwnedCrate {
    pub id: i32,
    pub name: String,
    pub email_notifications: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableMe {
    pub user: EncodablePrivateUser,
    pub owned_crates: Vec<OwnedCrate>,
}

/// The serialization format for the `User` model.
/// Same as public user, except for addition of
/// email field
#[derive(Deserialize, Serialize, Debug)]
pub struct EncodablePrivateUser {
    pub id: i32,
    pub login: String,
    pub email_verified: bool,
    pub email_verification_sent: bool,
    pub name: Option<String>,
    pub email: Option<String>,
    pub avatar: Option<String>,
    pub url: Option<String>,
    pub is_admin: bool,
    pub publish_notifications: bool,
}

impl EncodablePrivateUser {
    /// Converts this `User` model into an `EncodablePrivateUser` for JSON serialization.
    pub fn from(
        user: User,
        email: Option<String>,
        email_verified: bool,
        email_verification_sent: bool,
    ) -> Self {
        let User {
            id,
            name,
            gh_login,
            gh_avatar,
            is_admin,
            publish_notifications,
            ..
        } = user;
        let url = format!("https://github.com/{gh_login}");

        EncodablePrivateUser {
            id,
            email,
            email_verified,
            email_verification_sent,
            avatar: gh_avatar,
            login: gh_login,
            name,
            url: Some(url),
            is_admin,
            publish_notifications,
        }
    }
}

/// The serialization format for the `User` model.
/// Same as private user, except no email field
#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
pub struct EncodablePublicUser {
    pub id: i32,
    pub login: String,
    pub name: Option<String>,
    pub avatar: Option<String>,
    pub url: String,
}

/// Converts a `User` model into an `EncodablePublicUser` for JSON serialization.
impl From<User> for EncodablePublicUser {
    fn from(user: User) -> Self {
        let User {
            id,
            name,
            gh_login,
            gh_avatar,
            ..
        } = user;
        let url = format!("https://github.com/{gh_login}");
        EncodablePublicUser {
            id,
            avatar: gh_avatar,
            login: gh_login,
            name,
            url,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct EncodableAuditAction {
    pub action: String,
    pub user: EncodablePublicUser,
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableVersion {
    pub id: i32,
    #[serde(rename = "crate")]
    pub krate: String,
    pub num: String,
    pub dl_path: String,
    pub readme_path: String,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    // NOTE: Used by shields.io, altering `downloads` requires a PR with shields.io
    pub downloads: i32,
    pub features: serde_json::Value,
    pub yanked: bool,
    pub yank_message: Option<String>,
    pub lib_links: Option<String>,
    // NOTE: Used by shields.io, altering `license` requires a PR with shields.io
    pub license: Option<String>,
    pub links: EncodableVersionLinks,
    pub crate_size: i32,
    pub published_by: Option<EncodablePublicUser>,
    pub audit_actions: Vec<EncodableAuditAction>,
    pub checksum: String,
    pub rust_version: Option<String>,
    pub has_lib: Option<bool>,
    pub bin_names: Option<Vec<Option<String>>>,
    pub edition: Option<String>,
    pub description: Option<String>,
    pub homepage: Option<String>,
    pub documentation: Option<String>,
    pub repository: Option<String>,
}

impl EncodableVersion {
    pub fn from(
        version: Version,
        crate_name: &str,
        published_by: Option<User>,
        audit_actions: Vec<(VersionOwnerAction, User)>,
    ) -> Self {
        let Version {
            id,
            num,
            updated_at,
            created_at,
            downloads,
            features,
            yanked,
            yank_message,
            links: lib_links,
            license,
            crate_size,
            checksum,
            rust_version,
            has_lib,
            bin_names,
            edition,
            description,
            homepage,
            documentation,
            repository,
            ..
        } = version;

        let links = EncodableVersionLinks {
            dependencies: format!("/api/v1/crates/{crate_name}/{num}/dependencies"),
            version_downloads: format!("/api/v1/crates/{crate_name}/{num}/downloads"),
            authors: format!("/api/v1/crates/{crate_name}/{num}/authors"),
        };

        Self {
            dl_path: format!("/api/v1/crates/{crate_name}/{num}/download"),
            readme_path: format!("/api/v1/crates/{crate_name}/{num}/readme"),
            num,
            id,
            krate: crate_name.to_string(),
            updated_at,
            created_at,
            downloads,
            features,
            yanked,
            yank_message,
            lib_links,
            license,
            links,
            crate_size,
            checksum,
            rust_version,
            has_lib,
            bin_names,
            edition,
            description,
            homepage,
            documentation,
            repository,
            published_by: published_by.map(User::into),
            audit_actions: audit_actions
                .into_iter()
                .map(|(audit_action, user)| EncodableAuditAction {
                    action: audit_action.action.into(),
                    user: user.into(),
                    time: audit_action.time,
                })
                .collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EncodableVersionLinks {
    pub dependencies: String,
    pub version_downloads: String,
    pub authors: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GoodCrate {
    #[serde(rename = "crate")]
    pub krate: EncodableCrate,
    pub warnings: PublishWarnings,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PublishWarnings {
    pub invalid_categories: Vec<String>,
    pub invalid_badges: Vec<String>,
    pub other: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn category_dates_serializes_to_rfc3339() {
        let cat = EncodableCategory {
            id: "".to_string(),
            category: "".to_string(),
            slug: "".to_string(),
            description: "".to_string(),
            crates_cnt: 1,
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap()
                .and_utc(),
            subcategories: None,
            parent_categories: None,
        };
        let json = serde_json::to_string(&cat).unwrap();
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:11Z""#));
    }

    #[test]
    fn keyword_serializes_to_rfc3339() {
        let key = EncodableKeyword {
            id: "".to_string(),
            keyword: "".to_string(),
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap()
                .and_utc(),
            crates_cnt: 0,
        };
        let json = serde_json::to_string(&key).unwrap();
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:11Z""#));
    }

    #[test]
    fn version_serializes_to_rfc3339() {
        let ver = EncodableVersion {
            id: 1,
            krate: "".to_string(),
            num: "".to_string(),
            dl_path: "".to_string(),
            readme_path: "".to_string(),
            updated_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap()
                .and_utc(),
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 12)
                .unwrap()
                .and_utc(),
            downloads: 0,
            features: serde_json::from_str("{}").unwrap(),
            yanked: false,
            yank_message: None,
            license: None,
            lib_links: None,
            links: EncodableVersionLinks {
                dependencies: "".to_string(),
                version_downloads: "".to_string(),
                authors: "".to_string(),
            },
            crate_size: 1234,
            checksum: String::new(),
            rust_version: None,
            has_lib: None,
            bin_names: None,
            published_by: None,
            edition: None,
            description: None,
            homepage: None,
            documentation: None,
            repository: None,
            audit_actions: vec![EncodableAuditAction {
                action: "publish".to_string(),
                user: EncodablePublicUser {
                    id: 0,
                    login: String::new(),
                    name: None,
                    avatar: None,
                    url: String::new(),
                },
                time: NaiveDate::from_ymd_opt(2017, 1, 6)
                    .unwrap()
                    .and_hms_opt(14, 23, 12)
                    .unwrap()
                    .and_utc(),
            }],
        };
        let json = serde_json::to_string(&ver).unwrap();
        assert_some!(json.as_str().find(r#""updated_at":"2017-01-06T14:23:11Z""#));
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:12Z""#));
        assert_some!(json.as_str().find(r#""time":"2017-01-06T14:23:12Z""#));
    }

    #[test]
    fn crate_serializes_to_rfc3399() {
        let crt = EncodableCrate {
            id: "".to_string(),
            name: "".to_string(),
            updated_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap()
                .and_utc(),
            versions: None,
            keywords: None,
            categories: None,
            badges: [],
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 12)
                .unwrap()
                .and_utc(),
            downloads: 0,
            recent_downloads: None,
            default_version: None,
            num_versions: 0,
            yanked: false,
            max_version: "".to_string(),
            newest_version: "".to_string(),
            max_stable_version: None,
            description: None,
            homepage: None,
            documentation: None,
            repository: None,
            links: EncodableCrateLinks {
                version_downloads: "".to_string(),
                versions: None,
                owners: None,
                owner_team: None,
                owner_user: None,
                reverse_dependencies: "".to_string(),
            },
            exact_match: false,
        };
        let json = serde_json::to_string(&crt).unwrap();
        assert_some!(json.as_str().find(r#""updated_at":"2017-01-06T14:23:11Z""#));
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:12Z""#));
    }

    #[test]
    fn crate_owner_invitation_serializes_to_rfc3339() {
        let inv = EncodableCrateOwnerInvitationV1 {
            invitee_id: 1,
            inviter_id: 2,
            invited_by_username: "".to_string(),
            crate_name: "".to_string(),
            crate_id: 123,
            created_at: NaiveDate::from_ymd_opt(2017, 1, 6)
                .unwrap()
                .and_hms_opt(14, 23, 11)
                .unwrap()
                .and_utc(),
            expires_at: NaiveDate::from_ymd_opt(2020, 10, 24)
                .unwrap()
                .and_hms_opt(16, 30, 00)
                .unwrap()
                .and_utc(),
        };
        let json = serde_json::to_string(&inv).unwrap();
        assert_some!(json.as_str().find(r#""created_at":"2017-01-06T14:23:11Z""#));
        assert_some!(json.as_str().find(r#""expires_at":"2020-10-24T16:30:00Z""#));
    }
}
