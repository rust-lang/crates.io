use anyhow::{Context, anyhow};
use std::path::PathBuf;
use std::{env, fs};

/// Writes the public GitHub SSH keys to the `$HOME/.ssh/known_hosts` file.
pub fn write_known_hosts_file() -> anyhow::Result<()> {
    let Some(home) = env::var_os("HOME").map(PathBuf::from) else {
        return Err(anyhow!("Failed to read HOME environment variable"));
    };

    let ssh_path = home.join(".ssh");
    fs::create_dir_all(&ssh_path).context("Failed to create `.ssh` directory")?;

    let known_hosts_path = ssh_path.join("known_hosts");
    fs::write(known_hosts_path, include_bytes!("./known_hosts"))
        .context("Failed to write `known_hosts` file")?;

    Ok(())
}
