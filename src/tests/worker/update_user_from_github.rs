use crate::util::TestApp;
use claims::{assert_err, assert_ok};
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
    dry_run: bool,
    existing_github_id: i64,
    existing_username: &'static str,
    github_mock: MockGitHubClient,
    expected_username: &'static str,
    expected_last_sync_updated: bool,
}

impl UpdateTest {
    async fn run(self) -> anyhow::Result<()> {
        let Self {
            dry_run,
            existing_github_id,
            existing_username,
            github_mock,
            expected_username,
            expected_last_sync_updated,
        } = self;

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

        let job = jobs::UpdateUserFromGithub {
            dry_run,
            account_id: oauth_github_before_update.account_id,
        };
        job.enqueue(&conn).await.unwrap();
        let job_result = app.try_run_pending_background_jobs().await;

        let oauth_github_after_update = oauth_github::table
            .filter(oauth_github::user_id.eq(u.id))
            .first::<OauthGithub>(&mut conn)
            .await
            .unwrap();
        assert_eq!(oauth_github_after_update.login, expected_username);
        if expected_last_sync_updated {
            assert_ne!(oauth_github_after_update.last_sync, last_sync_before_update);
        } else {
            assert_eq!(oauth_github_after_update.last_sync, last_sync_before_update);
        }

        // For now, we want to update the `User` record too
        let user_after_update = User::find(&conn, u.id).await.unwrap();
        assert_eq!(user_after_update.gh_login, expected_username);

        // Drain the failed job so the `TestAppInner::drop` empty-queue
        // post-condition is satisfied.
        diesel::delete(background_jobs::table)
            .execute(&mut conn)
            .await
            .unwrap();

        job_result
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
    let mut github_mock = MockGitHubClient::new();
    // What we have and what GitHub has agree
    github_mock
        .expect_current_user()
        .return_once(|_| Ok(github_user(GITHUB_ID, EXISTING_LOGIN)));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: EXISTING_LOGIN,
        // but still update last sync
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn yes_updates_needed() {
    let mut github_mock = MockGitHubClient::new();
    github_mock
        .expect_current_user()
        .return_once(|_| Ok(github_user(GITHUB_ID, "my-new-username")));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_deleted() {
    let mut github_mock = MockGitHubClient::new();
    // If GitHub returns 404, this user has deleted their account.
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn still_deleted() {
    let mut github_mock = MockGitHubClient::new();
    // GitHub still returns 404, they're still deleted
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        // We marked this user as deleted previously
        existing_username: "ghost_1",
        github_mock,
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_success_no_update() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token has been revoked or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("Not allowed"))));
    // Falling back to anonymous get_user_by_id shows no rename needed
    github_mock
        .expect_get_user_by_id()
        .return_once(|_| Ok(github_user(GITHUB_ID, EXISTING_LOGIN)));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_enterprise_user() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token has been revoked or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("Not allowed"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: "asmith_microsoft",
        github_mock,
        expected_username: "asmith_microsoft",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_success_yes_update() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token has been revoked or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("Not allowed"))));
    // Falling back to anonymous get_user_by_id shows yes rename needed
    github_mock
        .expect_get_user_by_id()
        .return_once(|_| Ok(github_user(GITHUB_ID, "updated-username")));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: "updated-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_not_found_deleted() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token has been revoked or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("Not allowed"))));
    // Falling back to anonymous get_user_by_id shows the user has been deleted
    github_mock
        .expect_get_user_by_id()
        .return_once(|_| Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_other_error_no_update() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token has been revoked or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Unauthorized(anyhow::anyhow!("Not allowed"))));
    // Falling back to anonymous get_user_by_id fails too; try again later
    github_mock
        .expect_get_user_by_id()
        .return_once(|_| Err(GitHubError::Other(anyhow::anyhow!("Over your rate limit"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;

    assert_err!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_forbidden_fallback_success_yes_update() {
    let mut github_mock = MockGitHubClient::new();
    // This could mean the user's oauth token needs to be refreshed or similar
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Forbidden(anyhow::anyhow!("Not allowed"))));
    // Falling back to anonymous get_user_by_id shows yes rename needed
    github_mock
        .expect_get_user_by_id()
        .return_once(|_| Ok(github_user(GITHUB_ID, "updated-username")));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: "updated-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unavailable() {
    let mut github_mock = MockGitHubClient::new();
    // If GitHub returns some error we haven't accounted for, we can't know anything about
    // this user. Try again later.
    github_mock
        .expect_current_user()
        .return_once(|_| Err(GitHubError::Other(anyhow::anyhow!("9% uptime is one nine"))));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;

    assert_err!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn undeleted() {
    let mut github_mock = MockGitHubClient::new();
    // Not sure how often this happens, but if we marked an account as ghost but we get a
    // valid user again, we should update their username
    github_mock
        .expect_current_user()
        .return_once(|_| Ok(github_user(GITHUB_ID, "my-new-username")));

    let result = UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: "ghost_1",
        github_mock,
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;

    assert_ok!(result);
}

#[tokio::test(flavor = "multi_thread")]
async fn dry_run_mode_doesnt_update() {
    let mut github_mock = MockGitHubClient::new();
    github_mock
        .expect_current_user()
        .return_once(|_| Ok(github_user(GITHUB_ID, "my-new-username")));

    let result = UpdateTest {
        dry_run: true,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_mock,
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;

    assert_ok!(result);
}
