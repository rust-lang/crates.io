use bon::Builder;
use chrono::{DateTime, Utc};
use diesel::dsl::sql;
use diesel::prelude::*;
use diesel::sql_types::Integer;
use diesel::upsert::excluded;
use diesel_async::{AsyncPgConnection, RunQueryDsl};
use serde::Serialize;

use crate::models::{Crate, CrateOwner, Email, Owner, OwnerKind};
use crate::schema::{crate_owners, emails, oauth_github, users};
use crates_io_diesel_helpers::lower;

/// The model representing a row in the `users` database table.
#[derive(Clone, Debug, HasQuery, Identifiable, Serialize)]
pub struct User {
    pub id: i32,
    pub name: Option<String>,
    pub gh_id: i32,
    pub gh_login: String,
    pub gh_avatar: Option<String>,
    #[serde(skip)]
    pub gh_encrypted_token: Vec<u8>,
    pub account_lock_reason: Option<String>,
    pub account_lock_until: Option<DateTime<Utc>>,
    pub is_admin: bool,
    pub publish_notifications: bool,
}

impl User {
    pub async fn find(mut conn: &AsyncPgConnection, id: i32) -> QueryResult<User> {
        User::query().find(id).first(&mut conn).await
    }

    pub async fn find_by_login(mut conn: &AsyncPgConnection, login: &str) -> QueryResult<User> {
        User::query()
            .filter(lower(users::gh_login).eq(login.to_lowercase()))
            .filter(users::gh_id.ne(-1))
            .order(users::gh_id.desc())
            .first(&mut conn)
            .await
    }

    pub async fn owning(krate: &Crate, mut conn: &AsyncPgConnection) -> QueryResult<Vec<Owner>> {
        let users = CrateOwner::by_owner_kind(OwnerKind::User)
            .inner_join(users::table)
            .select(User::as_select())
            .filter(crate_owners::crate_id.eq(krate.id))
            .load(&mut conn)
            .await?
            .into_iter()
            .map(Owner::User);

        Ok(users.collect())
    }

    /// Look up a user by their external OAuth identity.
    ///
    /// `provider` is the machine name of an OAuth provider (e.g., "github").
    /// `account_id` is the provider-native identifier as a string; each
    /// provider's storage table parses it into the column's native type
    /// (GitHub uses BIGINT; Bitbucket will use TEXT).
    ///
    /// Returns `Ok(None)` if no user matches. Returns `Ok(None)` (not an
    /// error) when the account_id fails to parse for a provider that
    /// expects a specific shape — the semantic is "is this a known user",
    /// not "is this input well-formed".
    pub async fn find_by_oauth_identity(
        conn: &mut AsyncPgConnection,
        provider: &str,
        account_id: &str,
    ) -> QueryResult<Option<User>> {
        match provider {
            // Must match `crates_io::oauth::github_provider::PROVIDER_NAME`.
            // Kept as a literal here becuase this crate can't depend on the
            // main crate without creating a circular dependency.
            "github" => {
                let Ok(gh_id) = account_id.parse::<i64>() else {
                    tracing::debug!(
                        provider,
                        account_id,
                        "oauth identity lookup skipped: account_id not numeric",
                    );
                    return Ok(None);
                };
                users::table
                    .inner_join(oauth_github::table.on(oauth_github::user_id.eq(users::id)))
                    .filter(oauth_github::account_id.eq(gh_id))
                    .select(User::as_select())
                    .first(conn)
                    .await
                    .optional()
            }
            _ => Ok(None),
        }
    }

    /// Fetches the encrypted OAuth token stored in `oauth_github` for this user.
    ///
    /// All token reads now go through this table rather than `users.gh_encrypted_token`
    /// so that the read-path works correctly after the Tier 1 identity cutover.
    pub async fn github_encrypted_token(
        &self,
        conn: &mut AsyncPgConnection,
    ) -> QueryResult<Vec<u8>> {
        oauth_github::table
            .filter(oauth_github::user_id.eq(self.id))
            .select(oauth_github::encrypted_token)
            .first(conn)
            .await
    }

    /// Queries the database for the verified emails
    /// belonging to a given user
    pub async fn verified_email(
        &self,
        mut conn: &AsyncPgConnection,
    ) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .filter(emails::verified.eq(true))
            .first(&mut conn)
            .await
            .optional()
    }

    /// Queries for the email belonging to a particular user
    pub async fn email(&self, mut conn: &AsyncPgConnection) -> QueryResult<Option<String>> {
        Email::belonging_to(self)
            .select(emails::email)
            .first(&mut conn)
            .await
            .optional()
    }
}

/// Represents a new user record insertable to the `users` table
#[derive(Insertable, Debug, Builder)]
#[diesel(table_name = users, check_for_backend(diesel::pg::Pg))]
pub struct NewUser<'a> {
    pub gh_id: i32,
    pub gh_login: &'a str,
    pub name: Option<&'a str>,
    pub gh_avatar: Option<&'a str>,
    pub gh_encrypted_token: &'a [u8],
}

impl NewUser<'_> {
    /// Inserts the user into the database, or fails if the user already exists.
    ///
    /// Also inserts a corresponding `oauth_github` row so that the token
    /// read-path (which now reads from `oauth_github.encrypted_token` instead
    /// of `users.gh_encrypted_token`) works without a full OAuth login flow.
    pub async fn insert(&self, mut conn: &AsyncPgConnection) -> QueryResult<User> {
        let user = diesel::insert_into(users::table)
            .values(self)
            .returning(User::as_returning())
            .get_result(&mut conn)
            .await?;

        diesel::insert_into(oauth_github::table)
            .values((
                oauth_github::account_id.eq(user.gh_id as i64),
                oauth_github::user_id.eq(user.id),
                oauth_github::login.eq(&user.gh_login),
                oauth_github::encrypted_token.eq(&user.gh_encrypted_token),
            ))
            .on_conflict(oauth_github::account_id)
            // Update the token on conflict so the token read-path (which now
            // reads from oauth_github.encrypted_token) always has a fresh value.
            // do_nothing() would silently skip the update, leaving a stale token.
            .do_update()
            .set(oauth_github::encrypted_token.eq(excluded(oauth_github::encrypted_token)))
            .execute(&mut conn)
            .await?;

        Ok(user)
    }

    /// Inserts the user into the database, or updates an existing one.
    pub async fn insert_or_update(&self, mut conn: &AsyncPgConnection) -> QueryResult<User> {
        diesel::insert_into(users::table)
            .values(self)
            // We need the `WHERE gh_id > 0` condition here because `gh_id` set
            // to `-1` indicates that we were unable to find a GitHub ID for
            // the associated GitHub login at the time that we backfilled
            // GitHub IDs. Therefore, there are multiple records in production
            // that have a `gh_id` of `-1` so we need to exclude those when
            // considering uniqueness of `gh_id` values. The `> 0` condition isn't
            // necessary for most fields in the database to be used as a conflict
            // target :)
            .on_conflict(sql::<Integer>("(gh_id) WHERE gh_id > 0"))
            .do_update()
            .set((
                users::gh_login.eq(excluded(users::gh_login)),
                users::name.eq(excluded(users::name)),
                users::gh_avatar.eq(excluded(users::gh_avatar)),
                users::gh_encrypted_token.eq(excluded(users::gh_encrypted_token)),
            ))
            .returning(User::as_returning())
            .get_result(&mut conn)
            .await
    }
}

/// Represents an OAuth GitHub account record linked to a user record.
/// Stored in the `oauth_github` table.
#[derive(Associations, Identifiable, Selectable, Queryable, Debug, Clone)]
#[diesel(
    table_name = oauth_github,
    check_for_backend(diesel::pg::Pg),
    primary_key(account_id),
    belongs_to(User),
)]
pub struct OauthGithub {
    /// In the process of being migrated from `users.gh_id`.
    /// GitHub API docs describe this type as int64
    pub account_id: i64,
    /// In the process of being migrated from `users.gh_avatar`.
    pub avatar: Option<String>,
    /// In the process of being migrated from `users.gh_encrypted_token`.
    pub encrypted_token: Vec<u8>,
    /// In the process of being migrated from `users.gh_login`.
    pub login: String,
    /// Foreign key to the `users` table.
    pub user_id: i32,
}

/// Represents a new crates.io user to GitHub user OAuth link to be inserted into the
/// `oauth_github` table.
#[derive(Insertable, Debug, Builder)]
#[diesel(
    table_name = oauth_github,
    check_for_backend(diesel::pg::Pg),
    primary_key(account_id),
    belongs_to(User),
)]
pub struct NewOauthGithub<'a> {
    pub account_id: i64,           // corresponds to users.gh_id
    pub avatar: Option<&'a str>,   // corresponds to users.gh_avatar
    pub encrypted_token: &'a [u8], // corresponds to users.gh_encrypted_token
    pub login: &'a str,            // corresponds to users.gh_login
    pub user_id: i32,
}

impl NewOauthGithub<'_> {
    /// Inserts the associated GitHub account info into the database, or updates an existing record.
    ///
    /// GitHub `account_id` is the primary key of the `oauth_github` table, and comes from GitHub.
    ///
    /// Each GitHub account ID can only be associated with one crates.io account, so that we know
    /// who to log in when we get a GitHub oAuth response.
    ///
    /// If this function gets an `account_id` conflict, it does not and should not update the
    /// `user_id` to that of the currently-logged-in crates.io user's ID because that would mean
    /// that GitHub account has already been associated with a different crates.io account. In that
    /// case, the currently-logged-in crates.io user should be logged out and the crates.io user
    /// already associated with this GitHub user should be logged in.
    ///
    /// We may eventually implement the ability to associate multiple GitHub accounts with one
    /// crates.io account.
    ///
    /// This function should be called if there is no current user and should update the encrypted
    /// token, login, or avatar if those have changed.
    pub async fn insert_or_update(&self, mut conn: &AsyncPgConnection) -> QueryResult<OauthGithub> {
        diesel::insert_into(oauth_github::table)
            .values(self)
            .on_conflict(oauth_github::account_id)
            .do_update()
            .set((
                oauth_github::encrypted_token.eq(excluded(oauth_github::encrypted_token)),
                oauth_github::login.eq(excluded(oauth_github::login)),
                oauth_github::avatar.eq(excluded(oauth_github::avatar)),
            ))
            .get_result(&mut conn)
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crates_io_test_db::TestDatabase;
    use diesel_async::RunQueryDsl;

    async fn setup() -> (TestDatabase, AsyncPgConnection) {
        let db = TestDatabase::new();
        let conn = db.async_connect().await;
        (db, conn)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn find_by_oauth_identity_returns_user_for_known_github_account() {
        let (_db, mut conn) = setup().await;

        let user_id = diesel::insert_into(users::table)
            .values((
                users::gh_id.eq(1001),
                users::gh_login.eq("alice"),
                users::gh_encrypted_token.eq(vec![0u8; 32]),
            ))
            .returning(users::id)
            .get_result::<i32>(&mut conn)
            .await
            .unwrap();

        diesel::insert_into(oauth_github::table)
            .values((
                oauth_github::account_id.eq(1001i64),
                oauth_github::user_id.eq(user_id),
                oauth_github::login.eq("alice"),
                oauth_github::encrypted_token.eq(vec![0u8; 32]),
            ))
            .execute(&mut conn)
            .await
            .unwrap();

        let result = User::find_by_oauth_identity(&mut conn, "github", "1001")
            .await
            .unwrap();

        assert!(result.is_some(), "expected Some(user), got None");
        assert_eq!(result.unwrap().id, user_id);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn find_by_oauth_identity_returns_none_for_unknown_provider() {
        let (_db, mut conn) = setup().await;

        let result = User::find_by_oauth_identity(&mut conn, "bitbucket", "some-account")
            .await
            .unwrap();

        assert!(result.is_none(), "expected None for unknown provider, got {result:?}");
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn find_by_oauth_identity_rejects_non_numeric_github_account_id() {
        let (_db, mut conn) = setup().await;

        let result = User::find_by_oauth_identity(&mut conn, "github", "not-a-number")
            .await
            .unwrap();

        assert!(
            result.is_none(),
            "expected Ok(None) for non-numeric github account_id, got {result:?}"
        );
    }
}
