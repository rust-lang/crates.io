use std::env;
use std::os::unix::process::CommandExt;
use std::process::{self, Command};

/// Replaces the compatibility process with the sibling `crates-io` executable.
pub fn forward(subcommand: Option<&str>) -> ! {
    let current_exe = env::current_exe().unwrap_or_else(|error| {
        eprintln!("failed to locate the compatibility executable: {error}");
        process::exit(1);
    });
    let crates_io = current_exe.with_file_name("crates-io");

    let mut command = Command::new(&crates_io);
    if let Some(subcommand) = subcommand {
        command.arg(subcommand);
    }
    command.args(env::args_os().skip(1));

    let error = command.exec();
    eprintln!("failed to execute `{}`: {error}", crates_io.display());
    process::exit(1);
}
