use crates_io_team_repo::{TeamRepo, TeamRepoImpl};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let team_repo = TeamRepoImpl::default();
    let permission = team_repo.get_permission("crates_io_admin").await?;
    println!("{permission:#?}");
    Ok(())
}
