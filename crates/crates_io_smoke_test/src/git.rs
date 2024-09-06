use crate::exit_status_ext::ExitStatusExt;
use std::path::Path;
use tokio::process::Command;

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
