use std::collections::{HashMap, HashSet};
use std::env;
use std::fs::File;
use std::io::prelude::*;
use std::io::{self, BufReader, BufStream};
use std::net::{TcpListener, TcpStream};
use std::path::PathBuf;
use std::str;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;

use curl::http;

// A "bomb" so when the test task exists we know when to shut down
// the server and fail if the subtask failed.
pub struct Bomb {
    accept: TcpListener,
    quit: Sender<()>,
    rx: Receiver<()>,
    iorx: Sink,
}

struct Sink(Arc<Mutex<Vec<u8>>>);

impl Write for Sink {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        Write::write(&mut *self.0.lock().unwrap(), data)
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

impl Drop for Bomb {
    fn drop(&mut self) {
        t!(self.quit.send(()));
        drop(TcpStream::connect(&t!(self.accept.socket_addr())));
        let res = self.rx.recv();
        let stderr = str::from_utf8(&self.iorx.0.lock().unwrap()).unwrap()
                         .to_string();
        match res {
            Err(..) if !thread::panicking() => {
                panic!("server subtask failed: {}", stderr)
            }
            _ => {
                if stderr.len() > 0 {
                    println!("server subtask failed: {}", stderr)
                }
            }
        }
    }
}

pub fn proxy() -> (String, Bomb) {
    let me = thread::current().name().unwrap().to_string();
    let record = env::var("RECORD").is_ok();

    let a = t!(TcpListener::bind("127.0.0.1:0"));
    let ret = format!("http://{}", t!(a.socket_addr()));
    let (tx, rx) = channel();

    let data = PathBuf::new(file!()).parent().unwrap().join("http-data")
                                    .join(&me.replace("::", "_"));
    println!("{:?}", data);
    let record = record && !data.exists();
    let a2 = t!(a.try_clone());

    let sink = Arc::new(Mutex::new(Vec::new()));
    let mut sink2 = Sink(sink.clone());

    let (quittx, quitrx) = channel();

    thread::spawn(move|| {
        let mut file = None;
        for socket in a.incoming() {
            if quitrx.try_recv().is_ok() { break }
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
        match file {
            Some(ref mut f) => {
                let mut s = String::new();
                t!(f.read_line(&mut s));
                assert_eq!(s, "");
            }
            None => {}
        }
        tx.send(()).unwrap();
    });

    (ret, Bomb { accept: a2, rx: rx, iorx: Sink(sink), quit: quittx })
}

fn record_http(mut socket: TcpStream, data: &mut BufStream<File>) {
    let mut request = Vec::new();
    let http_response = send((&mut socket).tee(&mut request));

    let mut response = Vec::new();
    respond(http_response, socket.broadcast(&mut response));

    t!(write!(data, "===REQUEST {}\n{}\n===RESPONSE {}\n{}\n",
              request.len(),
              str::from_utf8(&request).unwrap(),
              response.len(),
              str::from_utf8(&response).unwrap()));

    fn send<R: Read>(rdr: R) -> http::Response {
        let mut socket = BufReader::new(rdr);
        let method;
        let url;
        let mut headers = HashMap::new();
        {
            let mut lines = (&mut socket).lines();
            let line = t!(lines.next().unwrap());
            let mut parts = line.as_slice().split(' ');
            method = parts.next().unwrap().to_string();
            url = parts.next().unwrap().replace("http://", "https://");

            for line in lines {
                let line = t!(line);
                if line.len() < 3 { break }
                let mut parts = line.as_slice().splitn(1, ':');
                headers.insert(parts.next().unwrap().to_string(),
                               parts.next().unwrap()[1..].to_string());
            }
        }

        let mut handle = http::handle();
        let mut req = match method.as_slice() {
            "PUT" => handle.put(url, &mut socket),
            "POST" => handle.post(url, &mut socket),
            "DELETE" => handle.delete(url),
            "GET" => handle.get(url),
            _ => panic!("unknown method: {}", method),
        };
        for (k, v) in headers.iter() {
            let v = v.as_slice().trim();
            match k.as_slice() {
                "Content-Length" => req = req.content_length(v.parse().unwrap()),
                "Content-Type" => req = req.content_type(v),
                "Transfer-Encoding" => {}
                k => req = req.header(k, v),
            }
        }
        t!(req.exec())
    }

    fn respond<W: Write>(response: http::Response, mut socket: W) {
        t!(socket.write_all(format!("HTTP/1.1 {}\r\n",
                                    response.get_code()).as_bytes()));
        for (k, v) in response.get_headers().iter() {
            if k.as_slice() == "transfer-encoding" { continue }
            for v in v.iter() {
                t!(socket.write_all(k.as_bytes()));
                t!(socket.write_all(b": "));
                t!(socket.write_all(v.as_bytes()));
                t!(socket.write_all(b"\r\n"));
            }
        }
        t!(socket.write_all(b"\r\n"));
        t!(socket.write_all(response.get_body()));
    }
}

fn replay_http(socket: TcpStream, data: &mut BufStream<File>,
               stdout: &mut Write) {
    let mut writer = socket.try_clone().unwrap();
    let socket = BufReader::new(socket);

    let mut request = String::new();
    t!(data.read_line(&mut request));
    let mut request = request.split(' ');
    assert_eq!(request.next().unwrap(), "===REQUEST");
    let request_size = request.next().unwrap().trim().parse().unwrap();

    let mut expected = Vec::new();
    t!(data.take(request_size).read_to_end(&mut expected));
    let mut expected_lines = SliceExt::split(&expected[..], |b| *b == b'\n')
                                     .map(|s| str::from_utf8(s).unwrap())
                                     .map(|s| format!("{}", s));
    let mut actual_lines = socket.lines().map(|s| s.unwrap());

    // validate the headers
    let mut expected: HashSet<String> = expected_lines.by_ref()
                                                      .take_while(|l| l.len() > 2)
                                                      .collect();
    let mut found = HashSet::new();
    t!(write!(stdout, "expecting: {:?}", expected));
    for line in actual_lines.by_ref().take_while(|l| l.len() > 2) {
        t!(write!(stdout, "received: {}", line.as_slice().trim()));
        if !found.insert(line.clone()) { continue }
        if expected.remove(&line) { continue }
        if line.starts_with("Date:") { continue }
        if line.starts_with("Authorization:") { continue }
        panic!("unexpected header: {}", line);
    }
    for line in expected.iter() {
        if line.starts_with("Date:") { continue }
        if line.starts_with("Authorization:") { continue }
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
    let lines = SliceExt::split(&response[..], |b| *b == b'\n')
                        .map(|s| str::from_utf8(s).unwrap());
    for line in lines {
        if line.starts_with("Date:") { continue }
        writer.write_all(line.as_bytes()).unwrap();
        writer.write_all(b"\r\n").unwrap();
    }
    data.read_line(&mut String::new()).unwrap();
}
