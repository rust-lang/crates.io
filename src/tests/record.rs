use crate::new_user;
use cargo_registry::models::NewUser;
use std::{
    borrow::Cow,
    collections::HashSet,
    fs::{self, File},
    io::{self, prelude::*},
    path::PathBuf,
    pin::Pin,
    str,
    sync::{mpsc, Arc, Mutex, Once},
    task::{Context, Poll},
    thread,
};

use futures::{channel::oneshot, future, prelude::*};
use hyper::{body::to_bytes, Body, Error, Request, Response, Server, StatusCode, Uri};
use tokio::{
    net::{TcpListener, TcpStream},
    runtime,
};

// A "bomb" so when the test task exists we know when to shut down
// the server and fail if the subtask failed.
pub struct Bomb {
    iorx: Sink,
    quittx: Option<oneshot::Sender<()>>,
    #[allow(clippy::type_complexity)]
    thread: Option<thread::JoinHandle<Option<(Vec<u8>, PathBuf)>>>,
}

pub struct GhUser {
    pub login: &'static str,
    pub init: Once,
}

#[derive(Clone)]
struct Sink(Arc<Mutex<Vec<u8>>>);

impl<'a> Write for &'a Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.0.lock().unwrap(), data)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Bomb {
    fn drop(&mut self) {
        drop(self.quittx.take());
        let res = self.thread.take().unwrap().join();
        let stderr = str::from_utf8(&self.iorx.0.lock().unwrap())
            .unwrap()
            .to_string();
        match res {
            Err(..) if !thread::panicking() => panic!("server subtask failed: {}", stderr),
            Err(e) => {
                if !stderr.is_empty() {
                    println!("server subtask failed ({:?}): {}", e, stderr)
                }
            }
            Ok(_) if thread::panicking() => {}
            Ok(None) => {}
            Ok(Some((data, file))) => {
                t!(t!(File::create(&file)).write_all(&data));
            }
        }
    }
}

fn cache_file(name: &str) -> PathBuf {
    PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("http-data")
        .join(name)
}

enum Record {
    Capture(Vec<Exchange>, PathBuf),
    Replay(Vec<Exchange>),
}

pub fn proxy() -> (String, Bomb) {
    let me = thread::current().name().unwrap().to_string();
    let record = dotenv::var("RECORD").is_ok();

    let (url_tx, url_rx) = mpsc::channel();

    let data = cache_file(&me.replace("::", "_"));
    let record = if record && !data.exists() {
        Record::Capture(Vec::new(), data)
    } else if !data.exists() {
        Record::Replay(serde_json::from_slice(b"[]").unwrap())
    } else {
        let mut body = Vec::new();
        t!(t!(File::open(&data)).read_to_end(&mut body));
        Record::Replay(serde_json::from_slice(&body).unwrap())
    };

    let sink = Arc::new(Mutex::new(Vec::new()));
    let sink2 = Sink(Arc::clone(&sink));

    let (quittx, quitrx) = oneshot::channel();

    let thread = thread::spawn(move || {
        let mut rt = t!(runtime::Builder::new()
            .basic_scheduler()
            .enable_io()
            .build());

        let client = if let Record::Capture(_, _) = record {
            Some(hyper::Client::builder().build(hyper_tls::HttpsConnector::new()))
        } else {
            None
        };

        let mut listener = t!(rt.block_on(TcpListener::bind("127.0.0.1:0")));
        url_tx
            .send(format!("http://{}", t!(listener.local_addr())))
            .unwrap();

        let record = Arc::new(Mutex::new(record));
        let srv = Server::builder(hyper::server::accept::from_stream(listener.incoming()))
            .serve(Proxy {
                sink: sink2,
                record: Arc::clone(&record),
                client,
            })
            .with_graceful_shutdown(async {
                quitrx.await.ok();
            });

        rt.block_on(srv).ok();

        let record = record.lock().unwrap();
        match *record {
            Record::Capture(ref data, ref path) => {
                let data = t!(serde_json::to_string_pretty(data));
                Some((data.into_bytes(), path.clone()))
            }
            Record::Replay(..) => None,
        }
    });

    (
        url_rx.recv().unwrap(),
        Bomb {
            iorx: Sink(sink),
            quittx: Some(quittx),
            thread: Some(thread),
        },
    )
}

#[derive(Clone)]
struct Proxy {
    sink: Sink,
    record: Arc<Mutex<Record>>,
    client: Option<Client>,
}

impl tower_service::Service<Request<Body>> for Proxy {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Response<Body>, Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        match *self.record.lock().unwrap() {
            Record::Capture(_, _) => {
                let client = self.client.as_ref().unwrap().clone();
                let record2 = self.record.clone();
                Box::pin(async move {
                    let (response, exchange) = record_http(req, client).await?;
                    if let Record::Capture(ref mut d, _) = *record2.lock().unwrap() {
                        d.push(exchange);
                    }
                    Ok(response)
                })
            }
            Record::Replay(ref mut exchanges) => {
                Box::pin(replay_http(req, exchanges.remove(0), &mut &self.sink))
            }
        }
    }
}

impl<'a> tower_service::Service<&'a TcpStream> for Proxy {
    type Response = Proxy;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Proxy, Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: &'a TcpStream) -> Self::Future {
        Box::pin(future::ok(self.clone()))
    }
}

#[derive(Serialize, Deserialize)]
struct Exchange {
    request: RecordedRequest,
    response: RecordedResponse,
}

#[derive(Serialize, Deserialize)]
struct RecordedRequest {
    uri: String,
    method: String,
    headers: HashSet<(String, String)>,
    body: String,
}

#[derive(Serialize, Deserialize)]
struct RecordedResponse {
    status: u16,
    headers: HashSet<(String, String)>,
    body: String,
}

type Client = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;
type ResponseAndExchange = (Response<Body>, Exchange);

async fn record_http(req: Request<Body>, client: Client) -> Result<ResponseAndExchange, Error> {
    // Deconstruct the incoming request and await for the full body
    let (header_parts, body) = req.into_parts();
    let method = header_parts.method;
    let uri = header_parts.uri;
    let headers = header_parts.headers;
    let body = to_bytes(body).await?;

    // Save info on the incoming request for the exchange log
    let request = RecordedRequest {
        uri: uri.to_string(),
        method: method.to_string(),
        headers: headers
            .iter()
            .map(|h| (h.0.as_str().to_string(), h.1.to_str().unwrap().to_string()))
            .collect(),
        body: base64::encode(&body.to_vec()),
    };

    // Construct an outgoing request
    let uri = uri.to_string().replace("http://", "https://");
    let uri = uri.parse::<Uri>().unwrap();
    let mut req = Request::builder()
        .method(method.clone())
        .uri(uri)
        .body(body.into())
        .unwrap();
    *req.headers_mut() = headers.clone();

    // Deconstruct the incoming response and await for the full body
    let hyper_response = client.request(req).await?;
    let status = hyper_response.status();
    let headers = hyper_response.headers().clone();
    let body = to_bytes(hyper_response.into_body()).await?;

    // Save the response for the exchange log
    let response = RecordedResponse {
        status: status.as_u16(),
        headers: headers
            .iter()
            .map(|h| (h.0.as_str().to_string(), h.1.to_str().unwrap().to_string()))
            .collect(),
        body: base64::encode(&body.to_vec()),
    };

    // Construct an outgoing response
    let mut hyper_response = Response::builder()
        .status(status)
        .body(body.into())
        .unwrap();
    *hyper_response.headers_mut() = headers;

    Ok((hyper_response, Exchange { response, request }))
}

fn replay_http(
    req: Request<Body>,
    mut exchange: Exchange,
    stdout: &mut dyn Write,
) -> impl Future<Output = Result<Response<Body>, Error>> + Send {
    static IGNORED_HEADERS: &[&str] = &["authorization", "date", "cache-control"];

    assert_eq!(req.uri().to_string(), exchange.request.uri);
    assert_eq!(req.method().to_string(), exchange.request.method);
    t!(writeln!(
        stdout,
        "expecting: {:?}",
        exchange.request.headers
    ));
    for (name, value) in req.headers().iter() {
        let pair = (
            name.as_str().to_string(),
            value.to_str().unwrap().to_string(),
        );
        t!(writeln!(stdout, "received: {:?}", pair));
        if name == "user-agent" {
            assert_eq!(value, "crates.io (https://crates.io)");
            continue;
        }
        if IGNORED_HEADERS.contains(&name.as_str()) {
            continue;
        }
        if !exchange.request.headers.remove(&pair) {
            panic!("found {:?} but didn't expect it", pair);
        }
    }
    for (name, value) in exchange.request.headers.drain() {
        if IGNORED_HEADERS.contains(&name.as_str()) {
            continue;
        }
        panic!("didn't find header {:?}", (name, value));
    }

    async {
        assert_eq!(
            to_bytes(req.into_body()).await.unwrap(),
            base64::decode(&exchange.request.body).unwrap()
        );

        let mut builder = Response::builder();
        for (key, value) in exchange.response.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
        let body = base64::decode(exchange.response.body.as_bytes()).unwrap();
        let status = StatusCode::from_u16(exchange.response.status).unwrap();
        let response = builder.status(status).body(body.into()).unwrap();

        Ok(response)
    }
}

impl GhUser {
    pub fn user(&'static self) -> NewUser<'_> {
        self.init.call_once(|| self.init());
        let mut u = new_user(self.login);
        u.gh_access_token = Cow::Owned(self.token());
        u
    }

    fn filename(&self) -> PathBuf {
        cache_file(&format!("gh-{}", self.login))
    }

    fn token(&self) -> String {
        let mut token = String::new();
        File::open(&self.filename())
            .unwrap()
            .read_to_string(&mut token)
            .unwrap();
        token
    }

    fn init(&self) {
        if fs::metadata(&self.filename()).is_ok() {
            return;
        }

        let password = crate::env(&format!("GH_PASS_{}", self.login.replace("-", "_")));
        #[derive(Serialize)]
        struct Authorization {
            scopes: Vec<String>,
            note: String,
            client_id: String,
            client_secret: String,
        }
        let client = reqwest::blocking::Client::new();
        let req = client
            .post("https://api.github.com/authorizations")
            .json(&Authorization {
                scopes: vec!["read:org".to_string()],
                note: "crates.io test".to_string(),
                client_id: crate::env("GH_CLIENT_ID"),
                client_secret: crate::env("GH_CLIENT_SECRET"),
            })
            .basic_auth(self.login, Some(password));

        let response = t!(req
            .send()
            .and_then(reqwest::blocking::Response::error_for_status));

        #[derive(Deserialize)]
        struct Response {
            token: String,
        }
        let resp: Response = t!(response.json());
        File::create(&self.filename())
            .unwrap()
            .write_all(resp.token.as_bytes())
            .unwrap();
    }
}
