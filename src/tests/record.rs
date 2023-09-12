use std::{
    collections::HashSet,
    fs::File,
    future::Future,
    io::{self, prelude::*},
    path::PathBuf,
    pin::Pin,
    str,
    sync::{mpsc, Arc, Mutex},
    task::{Context, Poll},
    thread,
};

use base64::{engine::general_purpose, Engine};
use futures_channel::oneshot;
use futures_util::future;
use http::{header::CONTENT_TYPE, HeaderMap, HeaderValue};
use hyper::{
    body::to_bytes, server::conn::AddrStream, Body, Error, Request, Response, Server, StatusCode,
};
use tokio::runtime;

/// Headers that are ignored when capturing and replaying request logs
static IGNORED_HEADERS: &[&str] = &[
    "authorization",
    "date",
    "cache-control",
    "host",
    // This is explicitly checked in replay mode, but we want to ignore it on capture
    "user-agent",
];

/// A "bomb" so when the test task exists we know when to shut down
/// the server and fail if the subtask failed.
pub struct Bomb {
    iorx: Sink,
    quittx: Option<oneshot::Sender<()>>,
    #[allow(clippy::type_complexity)]
    thread: Option<thread::JoinHandle<Option<(Vec<u8>, PathBuf)>>>,
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
            Err(..) if !thread::panicking() => panic!("server subtask failed: {stderr}"),
            Err(e) => {
                if !stderr.is_empty() {
                    println!("server subtask failed ({e:?}): {stderr}")
                }
            }
            Ok(_) if thread::panicking() => {}
            Ok(Some((data, file))) if data != b"[]\n" => {
                assert_ok!(assert_ok!(File::create(file)).write_all(&data));
            }
            Ok(_) => {}
        }
    }
}

fn cache_file(name: &str) -> PathBuf {
    PathBuf::from(file!())
        .parent()
        .unwrap()
        .join("http-data")
        .join(name)
        .with_extension("json")
}

#[derive(Debug)]
enum Record {
    Replay(Vec<Exchange>),
}

pub fn proxy() -> (String, Bomb) {
    let me = thread::current().name().unwrap().to_string();

    let (url_tx, url_rx) = mpsc::channel();

    let path = cache_file(&me.replace("::", "_"));
    let record = if !path.exists() {
        Record::Replay(serde_json::from_slice(b"[]").unwrap())
    } else {
        let mut body = Vec::new();
        assert_ok!(assert_ok!(File::open(&path)).read_to_end(&mut body));
        Record::Replay(serde_json::from_slice(&body).unwrap())
    };

    let sink = Arc::new(Mutex::new(Vec::new()));
    let sink2 = Sink(Arc::clone(&sink));

    let (quittx, quitrx) = oneshot::channel();

    let thread = thread::spawn(move || {
        let rt = assert_ok!(runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build());
        // TODO: handle switching protocols for the client to http unconditionally, then detecting
        // the protocol to use in any upstream request in the proxy service.
        let record = Arc::new(Mutex::new(record));
        rt.block_on(async {
            let addr = ([127, 0, 0, 1], 0).into();
            let server = Server::bind(&addr).serve(Proxy {
                sink: sink2,
                record: Arc::clone(&record),
            });

            url_tx
                .send(format!("http://{}", server.local_addr()))
                .unwrap();

            server
                .with_graceful_shutdown(async {
                    quitrx.await.ok();
                })
                .await
                .unwrap();
        });

        let record = record.lock().unwrap();
        match *record {
            Record::Replay(ref remaining_exchanges) if !remaining_exchanges.is_empty() =>
                panic!(
                    "The HTTP proxy for this test received fewer requests than expected (remaining: {})",
                    remaining_exchanges.len()
                ),
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
            Record::Replay(ref mut exchanges) => {
                Box::pin(replay_http(req, exchanges.remove(0), &mut &self.sink))
            }
        }
    }
}

impl<'a> tower_service::Service<&'a AddrStream> for Proxy {
    type Response = Proxy;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Proxy, Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: &'a AddrStream) -> Self::Future {
        Box::pin(future::ok(self.clone()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Exchange {
    request: RecordedRequest,
    response: RecordedResponse,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecordedRequest {
    uri: String,
    method: String,
    #[serde(serialize_with = "sorted_headers")]
    headers: HashSet<(String, String)>,
    body: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RecordedResponse {
    status: u16,
    #[serde(serialize_with = "sorted_headers")]
    headers: HashSet<(String, String)>,
    body: String,
}

fn sorted_headers<S>(headers: &HashSet<(String, String)>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    use serde::ser::SerializeSeq;

    let mut headers = headers.clone().into_iter().collect::<Vec<_>>();
    headers.sort_by_cached_key(|(name, _)| name.clone());
    let mut seq = serializer.serialize_seq(Some(headers.len()))?;
    for header in &headers {
        seq.serialize_element(header)?;
    }
    seq.end()
}

fn replay_http(
    req: Request<Body>,
    mut exchange: Exchange,
    stdout: &mut dyn Write,
) -> impl Future<Output = Result<Response<Body>, Error>> + Send {
    debug!("<- {req:?}");
    assert_eq!(req.uri().to_string(), exchange.request.uri);
    assert_eq!(req.method().to_string(), exchange.request.method);
    assert_ok!(writeln!(
        stdout,
        "expecting: {:?}",
        exchange.request.headers
    ));
    for (name, value) in req.headers().iter() {
        let pair = (
            name.as_str().to_string(),
            value.to_str().unwrap().to_string(),
        );
        assert_ok!(writeln!(stdout, "received: {pair:?}"));
        if name == "user-agent" {
            assert_eq!(value, "crates.io (https://crates.io)");
            continue;
        }
        if IGNORED_HEADERS.contains(&name.as_str()) {
            continue;
        }
        if !exchange.request.headers.remove(&pair) {
            panic!("found {pair:?} but didn't expect it");
        }
    }
    for (name, value) in exchange.request.headers.drain() {
        if IGNORED_HEADERS.contains(&name.as_str()) {
            continue;
        }
        panic!("didn't find header {:?}", (name, value));
    }

    async move {
        let plain_text = is_plain_text(req.headers());
        let body = to_bytes(req.into_body()).await.unwrap();
        if plain_text {
            let body = String::from_utf8_lossy(&body);
            assert_eq!(body, exchange.request.body);
        } else {
            let body = general_purpose::STANDARD.encode(body);
            assert_eq!(body, exchange.request.body);
        }

        let mut builder = Response::builder();
        for (key, value) in exchange.response.headers {
            builder = builder.header(key.as_str(), value.as_str());
        }
        let body = general_purpose::STANDARD
            .decode(exchange.response.body.as_bytes())
            .unwrap();
        let status = StatusCode::from_u16(exchange.response.status).unwrap();
        let response = builder.status(status).body(body.into()).unwrap();

        debug!("-> {response:?}");
        Ok(response)
    }
}

fn is_plain_text(headers: &HeaderMap<HeaderValue>) -> bool {
    if let Some(header_value) = headers.get(CONTENT_TYPE) {
        if let Ok(value) = header_value.to_str() {
            return value == "text/plain";
        }
    }
    false
}
