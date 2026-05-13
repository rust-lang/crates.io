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
    dry_run: bool,
    existing_github_id: i64,
    existing_username: &'static str,
    github_current_user_response: Result<GitHubUser, GitHubError>,
    github_user_by_id_response: Result<GitHubUser, GitHubError>,
    expected_username: &'static str,
    expected_last_sync_updated: bool,
}

impl UpdateTest {
    async fn run(self) {
        let Self {
            dry_run,
            existing_github_id,
            existing_username,
            github_current_user_response,
            github_user_by_id_response,
            expected_username,
            expected_last_sync_updated,
        } = self;

        let mut github_mock = MockGitHubClient::new();
        github_mock
            .expect_current_user()
            .return_once(|_| github_current_user_response);
        github_mock
            .expect_get_user_by_id()
            .return_once(|_| github_user_by_id_response);

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

        let job = jobs::UpdateUserFromGithub::new(dry_run, oauth_github_before_update);
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
        dry_run: false,
        existing_github_id: GITHUB_ID,
        // What we have and what github has agree
        existing_username: EXISTING_LOGIN,
        github_current_user_response: Ok(github_user(GITHUB_ID, EXISTING_LOGIN)),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
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
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_current_user_response: Ok(github_user(GITHUB_ID, "my-new-username")),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn negative_github_id() {
    UpdateTest {
        dry_run: false,
        // The GitHub ID in our database is -1 because at some point we learned the GitHub user
        // was no longer valid
        existing_github_id: -1,
        existing_username: EXISTING_LOGIN,
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_current_user_response: Err(GitHubError::Other(anyhow::anyhow!(
            "current_user shouldn't be called"
        ))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn zero_github_id() {
    UpdateTest {
        dry_run: false,
        // Check that we're also treating 0 as invalid
        existing_github_id: 0,
        existing_username: EXISTING_LOGIN,
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_current_user_response: Err(GitHubError::Other(anyhow::anyhow!(
            "current_user shouldn't be called"
        ))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn negative_github_id_with_ghost_username() {
    UpdateTest {
        dry_run: false,
        // The GitHub ID in our database is negative because at some point we learned the GitHub
        // user was no longer valid
        existing_github_id: -9000,
        // We've already set this username to ghost, but set it and update `last_sync` anyway
        existing_username: "ghost_1",
        // We shouldn't even contact GitHub in this case, so error in case we do
        github_current_user_response: Err(GitHubError::Other(anyhow::anyhow!(
            "Shouldn't be called"
        ))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_deleted() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // If GitHub returns 404, this user has deleted their account.
        github_current_user_response: Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn still_deleted() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        // We marked this user as deleted previously
        existing_username: "ghost_1",
        // GitHub still returns 404, they're still deleted
        github_current_user_response: Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_success_no_update() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // This could mean the user's oauth token has been revoked or similar
        github_current_user_response: Err(GitHubError::Unauthorized(anyhow::anyhow!(
            "Not allowed"
        ))),
        // Falling back to anonymous get_user_by_id shows no rename needed
        github_user_by_id_response: Ok(github_user(GITHUB_ID, EXISTING_LOGIN)),
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_enterprise_user() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: "asmith_microsoft",
        // This could mean the user's oauth token has been revoked or similar
        github_current_user_response: Err(GitHubError::Unauthorized(anyhow::anyhow!(
            "Not allowed"
        ))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "asmith_microsoft",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_success_yes_update() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // This could mean the user's oauth token has been revoked or similar
        github_current_user_response: Err(GitHubError::Unauthorized(anyhow::anyhow!(
            "Not allowed"
        ))),
        // Falling back to anonymous get_user_by_id shows yes rename needed
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "updated-username")),
        expected_username: "updated-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_not_found_deleted() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // This could mean the user's oauth token has been revoked or similar
        github_current_user_response: Err(GitHubError::Unauthorized(anyhow::anyhow!(
            "Not allowed"
        ))),
        // Falling back to anonymous get_user_by_id shows the user has been deleted
        github_user_by_id_response: Err(GitHubError::NotFound(anyhow::anyhow!("404 Not Found"))),
        expected_username: "ghost_1",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unauthorized_fallback_other_error_no_update() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // This could mean the user's oauth token has been revoked or similar
        github_current_user_response: Err(GitHubError::Unauthorized(anyhow::anyhow!(
            "Not allowed"
        ))),
        // Falling back to anonymous get_user_by_id fails too; try again later
        github_user_by_id_response: Err(GitHubError::Other(anyhow::anyhow!(
            "Over your rate limit"
        ))),
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_forbidden_fallback_success_yes_update() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // This could mean the user's oauth token needs to be refreshed or similar
        github_current_user_response: Err(GitHubError::Forbidden(anyhow::anyhow!("Not allowed"))),
        // Falling back to anonymous get_user_by_id shows yes rename needed
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "updated-username")),
        expected_username: "updated-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn github_unavailable() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        // If GitHub returns some error we haven't accounted for, we can't know anything about
        // this user. Try again later.
        github_current_user_response: Err(GitHubError::Other(anyhow::anyhow!(
            "9% uptime is one nine"
        ))),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn undeleted() {
    UpdateTest {
        dry_run: false,
        existing_github_id: GITHUB_ID,
        existing_username: "ghost_1",
        // Not sure how often this happens, but if we marked an account as ghost but we get a
        // valid user again, we should update their username
        github_current_user_response: Ok(github_user(GITHUB_ID, "my-new-username")),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: "my-new-username",
        expected_last_sync_updated: true,
    }
    .run()
    .await;
}

#[tokio::test(flavor = "multi_thread")]
async fn dry_run_mode_doesnt_update() {
    UpdateTest {
        dry_run: true,
        existing_github_id: GITHUB_ID,
        existing_username: EXISTING_LOGIN,
        github_current_user_response: Ok(github_user(GITHUB_ID, "my-new-username")),
        // Shouldn't be called in this test
        github_user_by_id_response: Ok(github_user(GITHUB_ID, "wrong-user-info")),
        expected_username: EXISTING_LOGIN,
        expected_last_sync_updated: false,
    }
    .run()
    .await;
}
