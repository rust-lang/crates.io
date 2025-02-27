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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, utoipa::ToSchema)]
#[schema(as = LegacyCrateOwnerInvitation)]
pub struct EncodableCrateOwnerInvitationV1 {
    /// The ID of the user who was invited to be a crate owner.
    #[schema(example = 42)]
    pub invitee_id: i32,
    /// The ID of the user who sent the invitation.
    #[schema(example = 3)]
    pub inviter_id: i32,
    /// The username of the user who sent the invitation.
    #[schema(example = "ghost")]
    pub invited_by_username: String,
    /// The name of the crate that the user was invited to be an owner of.
    #[schema(example = "serde")]
    pub crate_name: String,
    /// The ID of the crate that the user was invited to be an owner of.
    #[schema(example = 123)]
    pub crate_id: i32,
    /// The date and time this invitation was created.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub created_at: DateTime<Utc>,
    /// The date and time this invitation will expire.
    #[schema(example = "2020-01-13T13:46:41Z")]
    pub expires_at: DateTime<Utc>,
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, utoipa::ToSchema)]
#[schema(as = CrateOwnerInvitation)]
pub struct EncodableCrateOwnerInvitation {
    /// The ID of the user who was invited to be a crate owner.
    #[schema(example = 42)]
    pub invitee_id: i32,
    /// The ID of the user who sent the invitation.
    #[schema(example = 3)]
    pub inviter_id: i32,
    /// The ID of the crate that the user was invited to be an owner of.
    #[schema(example = 123)]
    pub crate_id: i32,
    /// The name of the crate that the user was invited to be an owner of.
    #[schema(example = "serde")]
    pub crate_name: String,
    /// The date and time this invitation was created.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub created_at: DateTime<Utc>,
    /// The date and time this invitation will expire.
    #[schema(example = "2020-01-13T13:46:41Z")]
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

#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
pub struct OwnedCrate {
    /// The opaque identifier of the crate.
    #[schema(example = 123)]
    pub id: i32,

    /// The name of the crate.
    #[schema(example = "serde")]
    pub name: String,

    #[schema(deprecated)]
    pub email_notifications: bool,
}

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
pub struct EncodableMe {
    /// The authenticated user.
    pub user: EncodablePrivateUser,

    /// The crates that the authenticated user owns.
    #[schema(inline)]
    pub owned_crates: Vec<OwnedCrate>,
}

#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
#[schema(as = AuthenticatedUser)]
pub struct EncodablePrivateUser {
    /// An opaque identifier for the user.
    #[schema(example = 42)]
    pub id: i32,

    /// The user's login name.
    #[schema(example = "ghost")]
    pub login: String,

    /// Whether the user's email address has been verified.
    #[schema(example = true)]
    pub email_verified: bool,

    /// Whether the user's email address verification email has been sent.
    #[schema(example = true)]
    pub email_verification_sent: bool,

    /// The user's display name, if set.
    #[schema(example = "Kate Morgan")]
    pub name: Option<String>,

    /// The user's email address, if set.
    #[schema(example = "kate@morgan.dev")]
    pub email: Option<String>,

    /// The user's avatar URL, if set.
    #[schema(example = "https://avatars2.githubusercontent.com/u/1234567?v=4")]
    pub avatar: Option<String>,

    /// The user's GitHub profile URL.
    #[schema(example = "https://github.com/ghost")]
    pub url: Option<String>,

    /// Whether the user is a crates.io administrator.
    #[schema(example = false)]
    pub is_admin: bool,

    /// Whether the user has opted in to receive publish notifications via email.
    #[schema(example = true)]
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

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq, utoipa::ToSchema)]
#[schema(as = User)]
pub struct EncodablePublicUser {
    /// An opaque identifier for the user.
    #[schema(example = 42)]
    pub id: i32,

    /// The user's login name.
    #[schema(example = "ghost")]
    pub login: String,

    /// The user's display name, if set.
    #[schema(example = "Kate Morgan")]
    pub name: Option<String>,

    /// The user's avatar URL, if set.
    #[schema(example = "https://avatars2.githubusercontent.com/u/1234567?v=4")]
    pub avatar: Option<String>,

    /// The user's GitHub profile URL.
    #[schema(example = "https://github.com/ghost")]
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

#[derive(Deserialize, Serialize, Debug, utoipa::ToSchema)]
pub struct EncodableAuditAction {
    /// The action that was performed.
    #[schema(example = "publish")]
    pub action: String,

    /// The user who performed the action.
    pub user: EncodablePublicUser,

    /// The date and time the action was performed.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub time: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
#[schema(as = Version)]
pub struct EncodableVersion {
    /// An opaque identifier for the version.
    #[schema(example = 42)]
    pub id: i32,

    /// The name of the crate.
    #[serde(rename = "crate")]
    #[schema(example = "serde")]
    pub krate: String,

    /// The version number.
    #[schema(example = "1.0.0")]
    pub num: String,

    /// The API path to download the crate.
    #[schema(example = "/api/v1/crates/serde/1.0.0/download")]
    pub dl_path: String,

    /// The API path to download the crate's README file as HTML code.
    #[schema(example = "/api/v1/crates/serde/1.0.0/readme")]
    pub readme_path: String,

    /// The date and time this version was last updated (i.e. yanked or unyanked).
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub updated_at: DateTime<Utc>,

    /// The date and time this version was created.
    #[schema(example = "2019-12-13T13:46:41Z")]
    pub created_at: DateTime<Utc>,

    /// The total number of downloads for this version.
    #[schema(example = 123_456)]
    pub downloads: i32,

    /// The features defined by this version.
    #[schema(value_type = Object)]
    pub features: serde_json::Value,

    /// Whether this version has been yanked.
    #[schema(example = false)]
    pub yanked: bool,

    /// The message given when this version was yanked, if any.
    #[schema(example = "Security vulnerability")]
    pub yank_message: Option<String>,

    /// The name of the native library this version links with, if any.
    #[schema(example = "git2")]
    pub lib_links: Option<String>,

    /// The license of this version of the crate.
    #[schema(example = "MIT")]
    pub license: Option<String>,

    /// Links to other API endpoints related to this version.
    pub links: EncodableVersionLinks,

    /// The size of the compressed crate file in bytes.
    #[schema(example = 1_234)]
    pub crate_size: i32,

    /// The user who published this version.
    ///
    /// This field may be `null` if the version was published before crates.io
    /// started recording this information.
    pub published_by: Option<EncodablePublicUser>,

    /// A list of actions performed on this version.
    #[schema(inline)]
    pub audit_actions: Vec<EncodableAuditAction>,

    /// The SHA256 checksum of the compressed crate file encoded as a
    /// hexadecimal string.
    #[schema(example = "e8dfc9d19bdbf6d17e22319da49161d5d0108e4188e8b680aef6299eed22df60")]
    pub checksum: String,

    /// The minimum version of the Rust compiler required to compile
    /// this version, if set.
    #[schema(example = "1.31")]
    pub rust_version: Option<String>,

    /// Whether this version can be used as a library.
    #[schema(example = true)]
    pub has_lib: Option<bool>,

    /// The names of the binaries provided by this version, if any.
    #[schema(example = json!([]))]
    pub bin_names: Option<Vec<Option<String>>>,

    /// The Rust Edition used to compile this version, if set.
    #[schema(example = "2021")]
    pub edition: Option<String>,

    /// The description of this version of the crate.
    #[schema(example = "A generic serialization/deserialization framework")]
    pub description: Option<String>,

    /// The URL to the crate's homepage, if set.
    #[schema(example = "https://serde.rs")]
    pub homepage: Option<String>,

    /// The URL to the crate's documentation, if set.
    #[schema(example = "https://docs.rs/serde")]
    pub documentation: Option<String>,

    /// The URL to the crate's repository, if set.
    #[schema(example = "https://github.com/serde-rs/serde")]
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

#[derive(Serialize, Deserialize, Debug, utoipa::ToSchema)]
#[schema(as = VersionLinks)]
pub struct EncodableVersionLinks {
    /// The API path to download this version's dependencies.
    #[schema(example = "/api/v1/crates/serde/1.0.0/dependencies")]
    pub dependencies: String,

    /// The API path to download this version's download numbers.
    #[schema(example = "/api/v1/crates/serde/1.0.0/downloads")]
    pub version_downloads: String,

    /// The API path to download this version's authors.
    #[schema(deprecated, example = "/api/v1/crates/serde/1.0.0/authors")]
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
