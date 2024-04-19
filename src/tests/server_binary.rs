use crate::builders::CrateBuilder;
use crate::util::ChaosProxy;
use anyhow::{Context, Error};
use crates_io::models::{NewUser, User};
use crates_io_test_db::TestDatabase;
use diesel::prelude::*;
use googletest::prelude::*;
use reqwest::blocking::{Client, Response};
use reqwest::StatusCode;
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::result::Result;
use std::sync::{mpsc::Sender, Arc};
use std::time::Duration;
use tokio::runtime::Runtime;
use url::Url;

const SERVER_BOOT_TIMEOUT: Duration = Duration::from_secs(30);

#[test]
fn normal_startup() {
    let server_bin = ServerBin::prepare().unwrap();
    initialize_dummy_crate(&mut server_bin.db().unwrap());

    let running_server = server_bin.start().unwrap();

    // Ensure the application correctly responds to download requests
    let resp = running_server
        .get("api/v1/crates/foo/1.0.0/download")
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = assert_some!(resp.headers().get("location"));
    let location = assert_ok!(location.to_str());
    assert_that!(location, ends_with("/crates/foo/foo-1.0.0.crate"));
}

#[cfg(feature = "slow-tests")]
#[test]
fn startup_without_database() {
    let server_bin = ServerBin::prepare().unwrap();
    initialize_dummy_crate(&mut server_bin.db().unwrap());

    // Break the networking *before* starting the binary, to ensure the binary can fully startup
    // without a database connection. Most of crates.io should not work when started without a
    // database, but unconditional redirects will work.
    server_bin.chaosproxy.break_networking().unwrap();

    let running_server = server_bin.start().unwrap();

    // Ensure unconditional redirects work.
    let resp = running_server
        .get("api/v1/crates/FOO/1.0.0/download")
        .unwrap();
    assert_eq!(resp.status(), StatusCode::FOUND);

    let location = assert_some!(resp.headers().get("location"));
    let location = assert_ok!(location.to_str());
    assert_that!(location, ends_with("/crates/FOO/FOO-1.0.0.crate"));
}

fn initialize_dummy_crate(conn: &mut PgConnection) {
    use crates_io::schema::users;

    let user: User = diesel::insert_into(users::table)
        .values(NewUser {
            gh_id: 0,
            gh_login: "user",
            ..NewUser::default()
        })
        .get_result(conn)
        .expect("failed to create dummy user");

    CrateBuilder::new("foo", user.id)
        .version("1.0.0")
        .build(conn)
        .expect("failed to create dummy crate");
}

struct ServerBin {
    _runtime: Runtime,
    chaosproxy: Arc<ChaosProxy>,
    db_url: String,
    env: HashMap<String, String>,
    test_database: TestDatabase,
}

impl ServerBin {
    fn prepare() -> Result<Self, Error> {
        let runtime = Runtime::new().expect("failed to create Tokio runtime");

        let mut env = dotenvy::vars().collect::<HashMap<_, _>>();
        // Bind a random port every time the server is started.
        env.insert("PORT".into(), "0".into());
        // Avoid creating too many database connections.
        env.insert("DB_PRIMARY_POOL_SIZE".into(), "2".into());
        env.insert("DB_REPLICA_POOL_SIZE".into(), "1".into());
        env.remove("DB_MIN_SIZE");
        // Other configuration variables needed for the application to boot.
        env.insert("WEB_ALLOWED_ORIGINS".into(), "http://localhost:8888".into());
        env.insert("SESSION_KEY".into(), "a".repeat(32));
        env.insert("GH_CLIENT_ID".into(), String::new());
        env.insert("GH_CLIENT_SECRET".into(), String::new());

        // Use a proxied fresh schema as the database url.
        let test_database = TestDatabase::new();
        let (chaosproxy, db_url) = ChaosProxy::proxy_database_url(test_database.url(), &runtime)?;
        env.remove("TEST_DATABASE_URL");
        env.insert("DATABASE_URL".into(), db_url.clone());
        env.insert("READ_ONLY_REPLICA_URL".into(), db_url.clone());

        Ok(ServerBin {
            _runtime: runtime,
            chaosproxy,
            db_url,
            env,
            test_database,
        })
    }

    fn db(&self) -> Result<PgConnection, Error> {
        Ok(PgConnection::establish(&self.db_url)?)
    }

    fn start(self) -> Result<RunningServer, Error> {
        let mut process = Command::new(env!("CARGO_BIN_EXE_server"))
            .env_clear()
            .envs(self.env)
            .env("RUST_LOG", "info")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let (port_send, port_recv) = std::sync::mpsc::channel();
        stream_processor(process.stdout.take().unwrap(), "stdout", Some(port_send));
        stream_processor(process.stderr.take().unwrap(), "stderr", None);

        // Possible causes for this to fail:
        // - the server binary failed to start
        // - the server binary requires a database connection now
        // - the server binary doesn't print "listening on port {port}" anymore
        let port: u16 = port_recv
            .recv_timeout(SERVER_BOOT_TIMEOUT)
            .context("the server took too much time to initialize")?
            .parse()?;

        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        Ok(RunningServer {
            process,
            port,
            http,
            _chaosproxy: self.chaosproxy,
            _test_database: self.test_database,
        })
    }
}

struct RunningServer {
    process: Child,
    port: u16,
    http: Client,

    // Keep these two items at the bottom in this order to drop everything in the correct order.
    _chaosproxy: Arc<ChaosProxy>,
    _test_database: TestDatabase,
}

impl RunningServer {
    fn get(&self, url: &str) -> Result<Response, Error> {
        Ok(self
            .http
            .get(format!("http://127.0.0.1:{}/{}", self.port, url))
            .header("User-Agent", "crates.io test suite")
            .send()?)
    }
}

impl Drop for RunningServer {
    fn drop(&mut self) {
        self.process
            .kill()
            .expect("failed to kill the server binary");
    }
}

fn stream_processor<R>(stream: R, kind: &'static str, port_send: Option<Sender<String>>)
where
    R: Read + Send + 'static,
{
    std::thread::spawn(move || {
        let stream = BufReader::new(stream);
        for line in stream.lines() {
            let line = match line {
                Ok(line) => line,
                // We receive an EOF when the process terminates
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => break,
                Err(err) => panic!("unexpected error while reading process {kind}: {err}"),
            };

            // If we expect the port number to be logged into this stream, look for it and send it
            // over the channel as soon as it's found.
            if let Some(port_send) = &port_send {
                let pattern = "Listening at ";
                if let Some(idx) = line.find(pattern) {
                    let start = idx + pattern.len();
                    let url = &line[start..];
                    let url = Url::parse(url).unwrap();
                    let port = url.port().unwrap();
                    port_send
                        .send(port.to_string())
                        .expect("failed to send the port to the test thread")
                }
            }

            println!("[server {kind}] {line}");
        }
    });
}
