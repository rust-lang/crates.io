use crate::exit_status_ext::ExitStatusExt;
use std::path::Path;
use tokio::process::Command;

#[allow(unstable_name_collisions)]
pub async fn set_user_name(project_path: &Path, name: &str) -> anyhow::Result<()> {
    Command::new("git")
        .args(["config", "user.name", name])
        .current_dir(project_path)
        .status()
        .await?
        .exit_ok()
        .map_err(Into::into)
}

#[allow(unstable_name_collisions)]
pub async fn set_user_email(project_path: &Path, email: &str) -> anyhow::Result<()> {
    Command::new("git")
        .args(["config", "user.email", email])
        .current_dir(project_path)
        .status()
        .await?
        .exit_ok()
        .map_err(Into::into)
}

#[allow(unstable_name_collisions)]
pub async fn add_all(project_path: &Path) -> anyhow::Result<()> {
    Command::new("git")
        .args(["add", "--all"])
        .current_dir(project_path)
        .status()
        .await?
        .exit_ok()
        .map_err(Into::into)
}

#[allow(unstable_name_collisions)]
pub async fn commit(project_path: &Path, message: &str) -> anyhow::Result<()> {
    Command::new("git")
        .args(["commit", "--message", message])
        .current_dir(project_path)
        .status()
        .await?
        .exit_ok()
        .map_err(Into::into)
}
