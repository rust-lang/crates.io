extern crate "cargo-registry" as cargo_registry;
extern crate civet;
extern crate green;
extern crate rustuv;

use std::os;
use civet::Server;

fn main() {
    let config = cargo_registry::Config {
        s3_bucket: env("S3_BUCKET"),
        s3_access_key: env("S3_ACCESS_KEY"),
        s3_secret_key: env("S3_SECRET_KEY"),
        s3_proxy: None,
        session_key: env("SESSION_KEY"),
        git_repo_bare: Path::new(env("GIT_REPO_BARE")),
        git_repo_checkout: Path::new(env("GIT_REPO_CHECKOUT")),
        gh_client_id: env("GH_CLIENT_ID"),
        gh_client_secret: env("GH_CLIENT_SECRET"),
        db_url: env("DATABASE_URL"),
        env: cargo_registry::Development,
        max_upload_size: 2 * 1024 * 1024,
    };
    let app = cargo_registry::App::new(&config);
    app.db_setup();
    let app = cargo_registry::middleware(app);

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
