#![deny(warnings)]

use cargo_registry::{boot, App, Env};
use jemalloc_ctl;
use std::{
    fs::File,
    sync::{mpsc::channel, Arc},
};

use civet::Server as CivetServer;
use conduit_hyper::Service as HyperService;
use reqwest::Client;

enum Server {
    Civet(CivetServer),
    Hyper(HyperService<conduit_middleware::MiddlewareBuilder>),
}

use Server::*;

fn main() {
    let _ = jemalloc_ctl::set_background_thread(true);

    // Initialize logging
    env_logger::init();

    let config = cargo_registry::Config::default();
    let client = Client::new();

    let app = App::new(&config, Some(client));
    let app = cargo_registry::build_handler(Arc::new(app));

    // On every server restart, ensure the categories available in the database match
    // the information in *src/categories.toml*.
    let categories_toml = include_str!("../boot/categories.toml");
    boot::categories::sync(categories_toml).unwrap();

    let heroku = dotenv::var("HEROKU").is_ok();
    let port = if heroku {
        8888
    } else {
        dotenv::var("PORT")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(8888)
    };
    let threads = dotenv::var("SERVER_THREADS")
        .map(|s| s.parse().expect("SERVER_THREADS was not a valid number"))
        .unwrap_or_else(|_| {
            if config.env == Env::Development {
                1
            } else {
                50
            }
        });

    let server = if dotenv::var("USE_HYPER").is_ok() {
        println!("Booting with a hyper based server");
        Hyper(HyperService::new(app, threads as usize))
    } else {
        println!("Booting with a civet based server");
        let mut cfg = civet::Config::new();
        cfg.port(port).threads(threads).keep_alive(true);
        Civet(CivetServer::start(cfg, app).unwrap())
    };

    println!("listening on port {}", port);

    // Creating this file tells heroku to tell nginx that the application is ready
    // to receive traffic.
    if heroku {
        File::create("/tmp/app-initialized").unwrap();
    }

    if let Hyper(server) = server {
        let addr = ([127, 0, 0, 1], port).into();
        server.run(addr);
    } else {
        // Civet server is already running, but we need to block the main thread forever
        // TODO: handle a graceful shutdown by just waiting for a SIG{INT,TERM}
        let (_tx, rx) = channel::<()>();
        rx.recv().unwrap();
    }
}
