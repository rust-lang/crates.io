use anyhow::{Context, Result};
use crates_io::util::gh_token_encryption::GitHubTokenEncryption;
use crates_io::{db, models::User};
use crates_io_database::schema::users;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use indicatif::{ProgressBar, ProgressIterator, ProgressStyle};
use secrecy::ExposeSecret;

#[derive(clap::Parser, Debug)]
#[command(
    name = "encrypt-github-tokens",
    about = "Encrypt existing plaintext GitHub tokens in the database.",
    long_about = "Backfill operation to encrypt existing plaintext GitHub tokens using AES-256-GCM. \
        This reads users with plaintext tokens but no encrypted tokens, encrypts them, and \
        updates the database with the encrypted versions."
)]
pub struct Opts {}

pub async fn run(_opts: Opts) -> Result<()> {
    println!("Starting GitHub token encryption backfill…");

    // Load encryption configuration
    let encryption = GitHubTokenEncryption::from_environment()
        .context("Failed to load encryption configuration")?;

    // Get database connection
    let mut conn = db::oneoff_connection()
        .await
        .context("Failed to establish database connection")?;

    // Query users with no encrypted tokens
    let users_to_encrypt = users::table
        .filter(users::gh_encrypted_token.is_null())
        .select(User::as_select())
        .load(&mut conn)
        .await
        .context("Failed to query users with plaintext tokens")?;

    let total_users = users_to_encrypt.len();
    if total_users == 0 {
        println!("Found no users that need token encryption. Exiting.");
        return Ok(());
    }

    println!("Found {total_users} users with plaintext tokens to encrypt");

    let pb = ProgressBar::new(total_users as u64);
    pb.set_style(ProgressStyle::with_template(
        "{bar:60} ({pos}/{len}, ETA {eta}) {msg}",
    )?);

    let mut encrypted_count = 0;
    let mut failed_count = 0;

    for user in users_to_encrypt.into_iter().progress_with(pb.clone()) {
        let user_id = user.id;
        let plaintext_token = user.gh_access_token.expose_secret();

        let encrypted_token = match encryption.encrypt(plaintext_token) {
            Ok(encrypted_token) => encrypted_token,
            Err(e) => {
                pb.suspend(|| eprintln!("Failed to encrypt token for user {user_id}: {e}"));
                failed_count += 1;
                continue;
            }
        };

        // Update the user with the encrypted token
        if let Err(e) = diesel::update(users::table.find(user_id))
            .set(users::gh_encrypted_token.eq(Some(encrypted_token)))
            .execute(&mut conn)
            .await
        {
            pb.suspend(|| eprintln!("Failed to update user {user_id}: {e}"));
            failed_count += 1;
            continue;
        }

        encrypted_count += 1;
    }

    pb.finish_with_message("Backfill completed!");
    println!("Successfully encrypted: {encrypted_count} tokens");

    if failed_count > 0 {
        eprintln!(
            "WARNING: {failed_count} tokens failed to encrypt. Please review the errors above."
        );
        std::process::exit(1);
    }

    // Verify the backfill by checking for any remaining unencrypted tokens
    let remaining_unencrypted = users::table
        .filter(users::gh_encrypted_token.is_null())
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .context("Failed to count remaining unencrypted tokens")?;

    if remaining_unencrypted > 0 {
        eprintln!("WARNING: {remaining_unencrypted} users still have unencrypted tokens");
        std::process::exit(1);
    }

    println!("Verification successful: All non-empty tokens have been encrypted!");
    Ok(())
}
