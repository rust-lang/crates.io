extern crate "cargo-registry" as cargo_registry;
extern crate civet;
extern crate green;
extern crate rustuv;
extern crate git2;

use std::os;
use std::sync::Arc;
use std::io::{mod, fs};
use civet::Server;

fn main() {
    let url = env("GIT_REPO_URL");
    let checkout = Path::new(env("GIT_REPO_CHECKOUT"));

    match git2::Repository::open(&checkout) {
        Ok(..) => {}
        Err(..) => {
            let _ = fs::rmdir_recursive(&checkout);
            fs::mkdir_recursive(&checkout, io::UserDir).unwrap();
            let config = git2::Config::open_default().unwrap();
            let url = url.as_slice();
            cargo_registry::git::with_authentication(url, &config, |f| {
                let cb = git2::RemoteCallbacks::new().credentials(f);
                try!(git2::build::RepoBuilder::new()
                                              .remote_callbacks(cb)
                                              .clone(url, &checkout));
                Ok(())
            }).unwrap();
        }
    }

    let config = cargo_registry::Config {
        s3_bucket: env("S3_BUCKET"),
        s3_access_key: env("S3_ACCESS_KEY"),
        s3_secret_key: env("S3_SECRET_KEY"),
        s3_proxy: None,
        session_key: env("SESSION_KEY"),
        git_repo_checkout: checkout,
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_registry::Development,
        max_upload_size: 2 * 1024 * 1024,
    };
    let app = cargo_registry::App::new(&config);
    if os::getenv("RESET").is_some() {
        app.db_setup();
    }
    let app = cargo_registry::middleware(Arc::new(app));

    let port = 8888;
    let _a = Server::start(civet::Config { port: port, threads: 8 }, app);
    println!("listening on port {}", port);
    wait_for_sigint();
}

fn env(s: &str) -> String {
    match os::getenv(s) {
        Some(s) => s,
        None => fail!("must have `{}` defined", s),
    }
}

// libnative doesn't have signal handling yet
fn wait_for_sigint() {
    use green::{SchedPool, PoolConfig, GreenTaskBuilder};
    use std::io::signal::{Listener, Interrupt};
    use std::task::TaskBuilder;

    let mut config = PoolConfig::new();
    config.event_loop_factory = rustuv::event_loop;

    let mut pool = SchedPool::new(config);
    TaskBuilder::new().green(&mut pool).spawn(proc() {
        let mut l = Listener::new();
        l.register(Interrupt).unwrap();
        l.rx.recv();
    });
    pool.shutdown();
}
