use crate::util::TestApp;
use crates_io::{
    controllers::session,
    models::{OauthGithub, User},
    schema::{background_jobs, oauth_github},
    util::gh_token_encryption::GitHubTokenEncryption,
    worker::jobs,
};
use crates_io_github::{GitHubError, GitHubUser, MockGitHubClient};
use crates_io_worker::BackgroundJob;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use std::sync::LazyLock;

const GITHUB_ID: i64 = 456789;
const EXISTING_LOGIN: &str = "my-login";
static ENCRYPTED_TOKEN: LazyLock<Vec<u8>> = LazyLock::new(|| {
    GitHubTokenEncryption::for_testing()
        .encrypt("some random token")
        .unwrap()
});

struct UpdateTest {
    existing_github_id: i64,
    existing_username: &'static str,
    github_response: Result<GitHubUser, GitHubError>,
    expected_username: &'static str,
    expected_last_sync_updated: bool,
}

impl UpdateTest {
    async fn run(self) {
        let Self {
            existing_github_id,
            existing_username,
            github_response,
            expected_username,
            expected_last_sync_updated,
        } = self;

        let mut github_mock = MockGitHubClient::new();
        github_mock
            .expect_current_user()
            .return_once(|_| github_response);

        let (app, _) = TestApp::full().with_github(github_mock).empty().await;
        let emails = &app.as_inner().emails;
        let mut conn = app.db_conn().await;

        let original_gh_user = github_user(existing_github_id, existing_username);
        let u =
            session::save_user_to_database(&original_gh_user, &ENCRYPTED_TOKEN, emails, &mut conn)
                .await
                .unwrap();

        let oauth_github_before_update = oauth_github::table
            .filter(oauth_github::user_id.eq(u.id))
            .first::<OauthGithub>(&mut conn)
            .await
            .unwrap();
        let last_sync_before_update = oauth_github_before_update.last_sync;

        let job = jobs::UpdateUserFromGithub::new(oauth_github_before_update);
        job.enqueue(&conn).await.unwrap();
        let _ = app.try_run_pending_background_jobs().await;

        let oauth_github_after_update = oauth_github::table
            .filter(oauth_github::user_id.eq(u.id))
            .first::<OauthGithub>(&mut conn)
            .await
            .unwrap();
        assert_eq!(expected_username, oauth_github_after_update.login);
        if expected_last_sync_updated {
            assert_ne!(last_sync_before_update, oauth_github_after_update.last_sync);
        } else {
            assert_eq!(last_sync_before_update, oauth_github_after_update.last_sync);
        }

        // For now, we want to update the `User` record too
        let user_after_update = User::find(&conn, u.id).await.unwrap();
        assert_eq!(expected_username, user_after_update.gh_login);

        // Drain the failed job so the `TestAppInner::drop` empty-queue
        // post-condition is satisfied.
        diesel::delete(background_jobs::table)
            .execute(&mut conn)
            .await
            .unwrap();
    }
}

fn github_user(id: i64, username: &str) -> GitHubUser {
    GitHubUser {
        login: username.into(),
        id: id as i32,
        avatar_url: None,
        email: None,
        name: None,
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn no_updates_needed() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        // What we have and what github has agree
        existing_username: EXISTING_LOGIN,
        github_response: Ok(github_user(GITHUB_ID, EXISTING_LOGIN)),
        expected_username: EXISTING_LOGIN,
        // but still update last sync
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn yes_updates_needed() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_response: Ok(github_user(GITHUB_ID, "my-new-username")),
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn negative_github_id() {
    UpdateTest {
        // The GitHub ID in our database is -1 because at some point we learned the GitHub user
        // was no longer valid
        existing_github_id: -1,
        existing_username: EXISTING_LOGIN,
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_response: Err(GitHubError::Other(anyhow::anyhow!("Shouldn't be called"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn zero_github_id() {
    UpdateTest {
        // Check that we're also treating 0 as invalid
        existing_github_id: 0,
        existing_username: EXISTING_LOGIN,
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_response: Err(GitHubError::Other(anyhow::anyhow!("Shouldn't be called"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn negative_github_id_with_ghost_username() {
    UpdateTest {
        // The GitHub ID in our database is negative because at some point we learned the GitHub
        // user was no longer valid
        existing_github_id: -9000,
        // We've already set this username to ghost, but set it and update `last_sync` anyway
        existing_username: "ghost_1",
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_response: Err(GitHubError::Other(anyhow::anyhow!("Shouldn't be called"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_deleted() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // If GitHub returns 404, this user has deleted their account.
        github_response: Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn still_deleted() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        // We marked this user as deleted previously
        existing_username: "ghost_1",
        // GitHub still returns 404, they're still deleted
        github_response: Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unavailable() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // If GitHub returns some error we haven't accounted for, we can't know anything about
        // this user. Try again later.
        github_response: Err(GitHubError::Other(anyhow::anyhow!("9% uptime is one nine"))),
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn undeleted() {
    UpdateTest {
        existing_github_id: GITHUB_ID,
        existing_username: "ghost_1",
        // Not sure how often this happens, but if we marked an account as ghost but we get a
        // valid user again, we should update their username
        github_response: Ok(github_user(GITHUB_ID, "my-new-username")),
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}
