extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio_core;
extern crate tokio_service;

use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::io;
use std::net;
use std::path::PathBuf;
use std::rc::Rc;
use std::str;
use std::sync::{Arc, Mutex, Once};
use std::thread;

use cargo_registry::user::NewUser;
use curl::easy::{Easy, List};
use self::futures::{Future, Stream};
use self::futures::sync::oneshot;
use self::hyper::server::Http;
use self::tokio_core::net::TcpListener;
use self::tokio_core::reactor::Core;
use self::tokio_service::Service;
use serde_json;

// A "bomb" so when the test task exists we know when to shut down
// the server and fail if the subtask failed.
pub struct Bomb {
    iorx: Sink,
    quittx: Option<oneshot::Sender<()>>,
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
            Err(e) => if stderr.len() > 0 {
                println!("server subtask failed ({:?}): {}", e, stderr)
            },
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
    let record = env::var("RECORD").is_ok();

    let a = t!(net::TcpListener::bind("127.0.0.1:0"));
    let ret = format!("http://{}", t!(a.local_addr()));

    let data = cache_file(&me.replace("::", "_"));
    let record = if record && !data.exists() {
        Record::Capture(Vec::new(), data)
    } else if !data.exists() {
        Record::Replay(serde_json::from_slice("[]".as_bytes()).unwrap())
    } else {
        let mut body = Vec::new();
        t!(t!(File::open(&data)).read_to_end(&mut body));
        Record::Replay(serde_json::from_slice(&body).unwrap())
    };

    let sink = Arc::new(Mutex::new(Vec::new()));
    let sink2 = Sink(sink.clone());

    let (quittx, quitrx) = oneshot::channel();

    let thread = thread::spawn(move || {
        let mut core = t!(Core::new());
        let handle = core.handle();
        let addr = t!(a.local_addr());
        let listener = t!(TcpListener::from_listener(a, &addr, &handle));
        let client = hyper::Client::configure()
            .connector(hyper_tls::HttpsConnector::new(4, &handle).unwrap())
            .build(&handle);


        let record = Rc::new(RefCell::new(record));
        let srv = listener.incoming().for_each(|(socket, addr)| {
            Http::new().bind_connection(
                &handle,
                socket,
                addr,
                Proxy {
                    sink: sink2.clone(),
                    record: record.clone(),
                    client: client.clone(),
                },
            );
            Ok(())
        });
        drop(core.run(srv.select2(quitrx)));

        let record = record.borrow();
        match *record {
            Record::Capture(ref data, ref path) => {
                let data = t!(serde_json::to_string(data));
                Some((data.into_bytes(), path.clone()))
            }
            Record::Replay(..) => None,
        }
    });

    (
        ret,
        Bomb {
            iorx: Sink(sink),
            quittx: Some(quittx),
            thread: Some(thread),
        },
    )
}

struct Proxy {
    sink: Sink,
    record: Rc<RefCell<Record>>,
    client: Client,
}

impl Service for Proxy {
    type Request = hyper::Request;
    type Response = hyper::Response;
    type Error = hyper::Error;
    type Future = Box<Future<Item = hyper::Response, Error = hyper::Error>>;

    fn call(&self, req: hyper::Request) -> Self::Future {
        match *self.record.borrow_mut() {
            Record::Capture(_, _) => {
                let record = self.record.clone();
                Box::new(record_http(req, &self.client).map(
                    move |(response, exchange)| {
                        if let Record::Capture(ref mut d, _) = *record.borrow_mut() {
                            d.push(exchange);
                        }
                        response
                    },
                ))
            }
            Record::Replay(ref mut exchanges) => {
                replay_http(req, exchanges.remove(0), &mut &self.sink)
            }
        }
    }
}

#[derive(Serialize, Deserialize)]
struct Exchange {
    request: Request,
    response: Response,
}

#[derive(Serialize, Deserialize)]
struct Request {
    uri: String,
    method: String,
    headers: HashSet<(String, String)>,
    body: Vec<u8>,
}

#[derive(Serialize, Deserialize)]
struct Response {
    status: u16,
    headers: HashSet<(String, String)>,
    body: Vec<u8>,
}

type Client = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

fn record_http(
    req: hyper::Request,
    client: &Client,
) -> Box<Future<Item = (hyper::Response, Exchange), Error = hyper::Error>> {
    let (method, uri, _version, headers, body) = req.deconstruct();

    let mut request = Request {
        uri: uri.to_string(),
        method: method.to_string(),
        headers: headers
            .iter()
            .map(|h| (h.name().to_string(), h.value_string()))
            .collect(),
        body: Vec::new(),
    };
    let body = body.concat2();

    let client = client.clone();
    let response = body.and_then(move |body| {
        request.body = body.to_vec();
        let uri = uri.to_string().replace("http://", "https://");
        let mut req = hyper::Request::new(method, uri.parse().unwrap());
        *req.headers_mut() = headers;
        req.set_body(body);
        client.request(req).map(|r| (r, request))
    });

    Box::new(response.and_then(|(hyper_response, request)| {
        let status = hyper_response.status();
        let headers = hyper_response.headers().clone();
        let mut response = Response {
            status: status.as_u16(),
            headers: headers
                .iter()
                .map(|h| (h.name().to_string(), h.value_string()))
                .collect(),
            body: Vec::new(),
        };

        hyper_response.body().concat2().map(move |body| {
            response.body = body.to_vec();
            let mut hyper_response = hyper::Response::new();
            hyper_response.set_body(body);
            hyper_response.set_status(status);
            *hyper_response.headers_mut() = headers;
            (
                hyper_response,
                Exchange {
                    response: response,
                    request: request,
                },
            )
        })
    }))
}

fn replay_http(
    req: hyper::Request,
    mut exchange: Exchange,
    stdout: &mut Write,
) -> Box<Future<Item = hyper::Response, Error = hyper::Error>> {
    assert_eq!(req.uri().to_string(), exchange.request.uri);
    assert_eq!(req.method().to_string(), exchange.request.method);
    t!(writeln!(
        stdout,
        "expecting: {:?}",
        exchange.request.headers
    ));
    for header in req.headers().iter() {
        let pair = (header.name().to_string(), header.value_string());
        t!(writeln!(stdout, "received: {:?}", pair));
        if header.name().starts_with("Date") {
            continue;
        }
        if header.name().starts_with("Authorization") {
            continue;
        }
        if !exchange.request.headers.remove(&pair) {
            panic!("found {:?} but didn't expect it", pair);
        }
    }
    for (name, value) in exchange.request.headers.drain() {
        if name.starts_with("Date") {
            continue;
        }
        if name.starts_with("Authorization") {
            continue;
        }
        panic!("didn't find header {:?}", (name, value));
    }
    let req_body = exchange.request.body;
    let verify_body = req.body().concat2().map(move |body| {
        assert_eq!(&body[..], &req_body[..]);
    });

    let mut response = hyper::Response::new();
    response.set_status(hyper::StatusCode::try_from(exchange.response.status).unwrap());
    for (key, value) in exchange.response.headers.into_iter() {
        response.headers_mut().append_raw(key, value);
    }
    response.set_body(exchange.response.body);

    Box::new(verify_body.map(|()| response))
}

impl GhUser {
    pub fn user(&'static self) -> NewUser {
        self.init.call_once(|| self.init());
        let mut u = ::new_user(self.login);
        u.gh_access_token = Cow::Owned(self.token());
        return u;
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
        return token;
    }

    fn init(&self) {
        if fs::metadata(&self.filename()).is_ok() {
            return;
        }

        let password = ::env(&format!("GH_PASS_{}", self.login.replace("-", "_")));
        #[derive(Serialize)]
        struct Authorization {
            scopes: Vec<String>,
            note: String,
            client_id: String,
            client_secret: String,
        }
        let mut handle = Easy::new();
        let body = serde_json::to_string(&Authorization {
            scopes: vec!["read:org".to_string()],
            note: "crates.io test".to_string(),
            client_id: ::env("GH_CLIENT_ID"),
            client_secret: ::env("GH_CLIENT_SECRET"),
        }).unwrap();

        t!(handle.url("https://api.github.com/authorizations"));
        t!(handle.username(self.login));
        t!(handle.password(&password));
        t!(handle.post(true));
        t!(handle.post_fields_copy(body.as_bytes()));

        let mut headers = List::new();
        headers.append("User-Agent: hello!").unwrap();
        t!(handle.http_headers(headers));

        let mut response = Vec::new();
        {
            let mut transfer = handle.transfer();
            t!(transfer.write_function(|data| {
                response.extend(data);
                Ok(data.len())
            }));
            t!(transfer.perform())
        }

        if t!(handle.response_code()) < 200 || t!(handle.response_code()) >= 300 {
            panic!("failed to get a 200 {}", String::from_utf8_lossy(&response));
        }

        #[derive(Deserialize)]
        struct Response {
            token: String,
        }
        let resp: Response = serde_json::from_str(str::from_utf8(&response).unwrap()).unwrap();
        File::create(&self.filename())
            .unwrap()
            .write_all(&resp.token.as_bytes())
            .unwrap();
    }
}
