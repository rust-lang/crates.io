use std::borrow::Cow;
use std::collections::HashSet;
use std::env;
use std::fs::File;
use std::fs;
use std::io::prelude::*;
use std::io::{self, BufReader};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex, Once};
use std::thread;

use bufstream::BufStream;
use cargo_registry::user::NewUser;
use curl::easy::{Easy, List, ReadError};
use rustc_serialize::json;

// A "bomb" so when the test task exists we know when to shut down
// the server and fail if the subtask failed.
pub struct Bomb {
    accept: TcpListener,
    quit: Sender<()>,
    rx: Receiver<()>,
    iorx: Sink,
}

pub struct GhUser {
    pub login: &'static str,
    pub init: Once,
}

struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.0.lock().unwrap(), data)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Bomb {
    fn drop(&mut self) {
        t!(self.quit.send(()));
        drop(TcpStream::connect(&t!(self.accept.local_addr())));
        let res = self.rx.recv();
        let stderr = str::from_utf8(&self.iorx.0.lock().unwrap())
            .unwrap()
            .to_string();
        match res {
            Err(..) if !thread::panicking() => panic!("server subtask failed: {}", stderr),
            _ => {
                if stderr.len() > 0 {
                    println!("server subtask failed: {}", stderr)
                }
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

pub fn proxy() -> (String, Bomb) {
    let me = thread::current().name().unwrap().to_string();
    let record = env::var("RECORD").is_ok();

    let a = t!(TcpListener::bind("127.0.0.1:0"));
    let ret = format!("http://{}", t!(a.local_addr()));
    let (tx, rx) = channel();

    let data = cache_file(&me.replace("::", "_"));
    let record = record && !data.exists();
    let a2 = t!(a.try_clone());

    let sink = Arc::new(Mutex::new(Vec::new()));
    let mut sink2 = Sink(sink.clone());

    let (quittx, quitrx) = channel();

    thread::spawn(move || {
        let mut file = None;
        for socket in a.incoming() {
            if quitrx.try_recv().is_ok() {
                break;
            }
            let socket = t!(socket);

            if file.is_none() {
                let io = t!(if record {
                    File::create(&data)
                } else {
                    File::open(&data)
                });
                file = Some(BufStream::new(io));
            }

            if record {
                record_http(socket, file.as_mut().unwrap());
            } else {
                replay_http(socket, file.as_mut().unwrap(), &mut sink2);
            }
        }
        if !record {
            if let Some(mut f) = file {
                let mut s = String::new();
                t!(f.read_line(&mut s));
                assert_eq!(s, "");
            }
        }
        tx.send(()).unwrap();
    });

    (
        ret,
        Bomb {
            accept: a2,
            rx: rx,
            iorx: Sink(sink),
            quit: quittx,
        },
    )
}

fn record_http(mut socket: TcpStream, data: &mut BufStream<File>) {
    let mut request = Vec::new();
    t!(socket.read_to_end(&mut request));
    let (handle, headers, body) = send(&request[..]);

    let mut response = Vec::new();
    respond(handle, headers, body, &mut response);
    t!(socket.write_all(&response));

    t!(write!(
        data,
        "===REQUEST {}\n{}\n===RESPONSE {}\n{}\n",
        request.len(),
        str::from_utf8(&request).unwrap(),
        response.len(),
        str::from_utf8(&response).unwrap()
    ));

    fn send<R: Read>(rdr: R) -> (Easy, Vec<Vec<u8>>, Vec<u8>) {
        let mut socket = BufReader::new(rdr);
        let method;
        let url;
        let mut headers = List::new();
        {
            let mut lines = (&mut socket).lines();
            let line = t!(lines.next().unwrap());
            let mut parts = line.split(' ');
            method = parts.next().unwrap().to_string();
            url = parts.next().unwrap().replace("http://", "https://");

            for line in lines {
                let line = t!(line);
                if line.len() < 3 {
                    break;
                }
                t!(headers.append(&line));
            }
        }

        let mut handle = Easy::new();
        t!(handle.url(&url));
        match &method[..] {
            "PUT" => t!(handle.put(true)),
            "POST" => t!(handle.post(true)),
            "GET" => t!(handle.get(true)),
            method => t!(handle.custom_request(method)),
        }
        t!(handle.http_headers(headers));

        let mut headers = Vec::new();
        let mut response = Vec::new();
        {
            let mut transfer = handle.transfer();
            t!(transfer.header_function(|header| {
                headers.push(header.to_owned());
                true
            }));
            t!(transfer.write_function(|data| {
                response.extend(data);
                Ok(data.len())
            }));
            t!(transfer.read_function(
                |buf| socket.read(buf).map_err(|_| ReadError::Abort),
            ));

            t!(transfer.perform());
        }

        (handle, headers, response)
    }

    fn respond<W: Write>(mut handle: Easy, headers: Vec<Vec<u8>>, body: Vec<u8>, mut socket: W) {
        t!(socket.write_all(
            format!("HTTP/1.1 {}\r\n", t!(handle.response_code())).as_bytes(),
        ));
        for header in headers {
            if header.starts_with(b"Transfer-Encoding: ") {
                continue;
            }
            t!(socket.write_all(&header));
            t!(socket.write_all(b"\r\n"));
        }
        t!(socket.write_all(b"\r\n"));
        t!(socket.write_all(&body));
    }
}

fn replay_http(socket: TcpStream, data: &mut BufStream<File>, stdout: &mut Write) {
    let mut writer = socket.try_clone().unwrap();
    let socket = BufReader::new(socket);

    let mut request = String::new();
    t!(data.read_line(&mut request));
    let mut request = request.split(' ');
    assert_eq!(request.next().unwrap(), "===REQUEST");
    let request_size = request.next().unwrap().trim().parse().unwrap();

    let mut expected = String::new();
    t!(data.take(request_size).read_to_string(&mut expected));
    let mut expected_lines = expected.lines().map(|s| s.to_string());
    let mut actual_lines = socket.lines().map(|s| s.unwrap());

    // validate the headers
    let mut expected: HashSet<String> = expected_lines
        .by_ref()
        .take_while(|l| l.len() > 2)
        .collect();
    let mut found = HashSet::new();
    t!(writeln!(stdout, "expecting: {:?}", expected));
    for line in actual_lines.by_ref().take_while(|l| l.len() > 2) {
        t!(writeln!(stdout, "received: {}", line.trim()));
        if !found.insert(line.clone()) {
            continue;
        }
        if expected.remove(&line) {
            continue;
        }
        if line.starts_with("Date:") {
            continue;
        }
        if line.starts_with("Authorization:") {
            continue;
        }
        panic!("unexpected header: {}", line);
    }
    for line in expected.iter() {
        if line.starts_with("Date:") {
            continue;
        }
        if line.starts_with("Authorization:") {
            continue;
        }
        panic!("didn't receive header: {}", line);
    }

    // TODO: validate the body

    data.read_line(&mut String::new()).unwrap();
    let mut response = String::new();
    data.read_line(&mut response).unwrap();
    let mut response = response.split(' ');
    assert_eq!(response.next().unwrap(), "===RESPONSE");
    let response_size = response.next().unwrap().trim().parse().unwrap();
    let mut response = Vec::new();
    data.take(response_size).read_to_end(&mut response).unwrap();
    let lines = <[_]>::split(&response[..], |b| *b == b'\n').map(|s| str::from_utf8(s).unwrap());
    for line in lines {
        if line.starts_with("Date:") {
            continue;
        }
        writer.write_all(line.as_bytes()).unwrap();
        writer.write_all(b"\r\n").unwrap();
    }
    data.read_line(&mut String::new()).unwrap();
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
        #[derive(RustcEncodable)]
        struct Authorization {
            scopes: Vec<String>,
            note: String,
            client_id: String,
            client_secret: String,
        }
        let mut handle = Easy::new();
        let url = format!(
            "https://{}:{}@api.github.com/authorizations",
            self.login,
            password
        );
        let body = json::encode(&Authorization {
            scopes: vec!["read:org".to_string()],
            note: "crates.io test".to_string(),
            client_id: ::env("GH_CLIENT_ID"),
            client_secret: ::env("GH_CLIENT_SECRET"),
        }).unwrap();

        t!(handle.url(&url));
        t!(handle.post(true));

        let mut headers = List::new();
        headers.append("User-Agent: hello!").unwrap();
        t!(handle.http_headers(headers));

        let mut body = body.as_bytes();
        let mut response = Vec::new();
        {
            let mut transfer = handle.transfer();
            t!(transfer.read_function(
                |buf| body.read(buf).map_err(|_| ReadError::Abort),
            ));
            t!(transfer.write_function(|data| {
                response.extend(data);
                Ok(data.len())
            }));
            t!(transfer.perform())
        }

        if t!(handle.response_code()) < 200 || t!(handle.response_code()) >= 300 {
            panic!("failed to get a 200 {}", String::from_utf8_lossy(&response));
        }

        #[derive(RustcDecodable)]
        struct Response {
            token: String,
        }
        let resp: Response = json::decode(str::from_utf8(&response).unwrap()).unwrap();
        File::create(&self.filename())
            .unwrap()
            .write_all(&resp.token.as_bytes())
            .unwrap();
    }
}
