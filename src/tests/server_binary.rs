use crate::builders::CrateBuilder;
use crate::util::{ChaosProxy, FreshSchema};
use anyhow::Error;
use cargo_registry::models::{NewUser, User};
use diesel::prelude::*;
use reqwest::blocking::{Client, Response};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read};
use std::process::{Child, Command, Stdio};
use std::sync::{mpsc::Sender, Arc};
use std::time::Duration;
use url::Url;

const SERVER_BOOT_TIMEOUT_SECONDS: u64 = 30;

#[test]
fn normal_startup() -> Result<(), Error> {
    let server_bin = ServerBin::prepare()?;
    initialize_dummy_crate(&server_bin.db()?);

    let running_server = server_bin.start()?;

    // Ensure the application correctly responds to download requests
    let resp = running_server.get("api/v1/crates/FOO/1.0.0/download")?;
    assert!(resp.status().is_redirection());
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()?
        .ends_with("/crates/foo/foo-1.0.0.crate"));

    Ok(())
}

#[test]
fn startup_without_database() -> Result<(), Error> {
    let server_bin = ServerBin::prepare()?;
    initialize_dummy_crate(&server_bin.db()?);

    // Break the networking *before* starting the binary, to ensure the binary can fully startup
    // without a database connection. Most of crates.io should not work when started without a
    // database, but unconditional redirects will work.
    server_bin.chaosproxy.break_networking();

    let running_server = server_bin.start()?;

    // Ensure unconditional redirects work.
    let resp = running_server.get("api/v1/crates/FOO/1.0.0/download")?;
    assert!(resp.status().is_redirection());
    assert!(resp
        .headers()
        .get("location")
        .unwrap()
        .to_str()?
        .ends_with("/crates/FOO/FOO-1.0.0.crate"));

    Ok(())
}

fn initialize_dummy_crate(conn: &PgConnection) {
    use cargo_registry::schema::users;

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
    chaosproxy: Arc<ChaosProxy>,
    db_url: String,
    env: HashMap<String, String>,
    fresh_schema: FreshSchema,
}

impl ServerBin {
    fn prepare() -> Result<Self, Error> {
        let mut env = dotenv::vars().collect::<HashMap<_, _>>();
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
        let fresh_schema = FreshSchema::new(env.get("TEST_DATABASE_URL").unwrap());
        let (chaosproxy, db_url) = ChaosProxy::proxy_database_url(fresh_schema.database_url())?;
        env.remove("TEST_DATABASE_URL");
        env.insert("DATABASE_URL".into(), db_url.clone());
        env.insert("READ_ONLY_REPLICA_URL".into(), db_url.clone());

        Ok(ServerBin {
            chaosproxy,
            db_url,
            env,
            fresh_schema,
        })
    }

    fn db(&self) -> Result<PgConnection, Error> {
        Ok(PgConnection::establish(&self.db_url)?)
    }

    fn start(self) -> Result<RunningServer, Error> {
        let mut process = Command::new(env!("CARGO_BIN_EXE_server"))
            .env_clear()
            .envs(self.env.into_iter())
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
            .recv_timeout(Duration::from_secs(SERVER_BOOT_TIMEOUT_SECONDS))
            .map_err(|_| anyhow::anyhow!("the server took too much time to initialize"))?
            .parse()?;

        let http = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;

        Ok(RunningServer {
            process,
            port,
            http,
            _chaosproxy: self.chaosproxy,
            _fresh_schema: self.fresh_schema,
        })
    }
}

struct RunningServer {
    process: Child,
    port: u16,
    http: Client,

    // Keep these two items at the bottom in this order to drop everything in the correct order.
    _chaosproxy: Arc<ChaosProxy>,
    _fresh_schema: FreshSchema,
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
                Err(err) => panic!("unexpected error while reading process {}: {}", kind, err),
            };

            // If we expect the port number to be logged into this stream, look for it and send it
            // over the channel as soon as it's found.
            if let Some(port_send) = &port_send {
                if let Some(url) = line.strip_prefix("Listening at ") {
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
