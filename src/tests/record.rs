use std::collections::{HashMap, HashSet};
use std::io::net::tcp::{TcpListener, TcpAcceptor, TcpStream};
use std::io::{ChanReader, ChanWriter, util, stdio};
use std::io::{Listener, Acceptor, File, BufferedReader, BufferedStream};
use std::os;
use std::str;
use std::task;

use curl::http;

// A "bomb" so when the test task exists we know when to shut down
// the server and fail if the subtask failed.
pub struct Bomb {
    accept: TcpAcceptor,
    rx: Receiver<()>,
    iorx: ChanReader,
}

impl Drop for Bomb {
    fn drop(&mut self) {
        self.accept.close_accept().unwrap();
        let res = self.rx.recv_opt();
        let stderr = self.iorx.read_to_string().unwrap();
        match res {
            Err(..) if !task::failing() => {
                fail!("server subtask failed: {}", stderr)
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
    let me = task::name().unwrap();
    let record = os::getenv("RECORD").is_some();

    let mut l = TcpListener::bind("127.0.0.1", 0).unwrap();
    let ret = format!("http://{}", l.socket_name().unwrap());
    let mut a = l.listen().unwrap();
    let (tx, rx) = channel();

    let data = Path::new(file!()).dir_path().join("http-data")
                                 .join(me.as_slice().replace("::", "_"));
    let file = if record {File::create(&data)} else {File::open(&data)};
    let a2 = a.clone();

    let (iotx, iorx) = channel();
    let (iotx, iorx) = (ChanWriter::new(iotx), ChanReader::new(iorx));

    spawn(proc() {
        stdio::set_stderr(box iotx.clone());
        stdio::set_stdout(box iotx);
        let mut file = Some(file);
        let mut data = None;
        for socket in a.incoming() {
            let socket = match socket { Ok(s) => s, Err(..) => break };

            if data.is_none() {
                data = Some(BufferedStream::new(file.take().unwrap().unwrap()));
            }

            if record {
                record_http(socket, data.as_mut().unwrap());
            } else {
                replay_http(socket, data.as_mut().unwrap());
            }
        }
        match data {
            Some(ref mut f) => assert!(f.read_line().is_err()),
            None => {}
        }
        tx.send(());
    });

    (ret, Bomb { accept: a2, rx: rx, iorx: iorx })
}

fn record_http(mut socket: TcpStream, data: &mut BufferedStream<File>) {
    let (tx, rx) = channel();
    let (tx, mut rx) = (ChanWriter::new(tx), ChanReader::new(rx));
    let http_response = send(util::TeeReader::new(&mut socket as &mut Reader, tx));
    let request = rx.read_to_end().unwrap();

    let (tx, rx) = channel();
    let (tx, mut rx) = (ChanWriter::new(tx), ChanReader::new(rx));
    let socket = box socket as Box<Writer + 'static>;
    let tx = box tx as Box<Writer + 'static>;
    respond(http_response, util::MultiWriter::new(vec![socket, tx]));
    let response = rx.read_to_end().unwrap();

    (write!(data, "===REQUEST {}\n{}\n===RESPONSE {}\n{}\n",
            request.len(),
            str::from_utf8(request.as_slice()).unwrap(),
            response.len(),
            str::from_utf8(response.as_slice()).unwrap())).unwrap();

    fn send<R: Reader>(rdr: R) -> http::Response {
        let mut socket = BufferedReader::new(rdr);
        let method;
        let url;
        let mut headers = HashMap::new();
        {
            let mut lines = socket.lines();
            let line = lines.next().unwrap().unwrap();
            let mut parts = line.as_slice().split(' ');
            method = parts.next().unwrap().to_string();
            url = parts.next().unwrap().replace("http://", "https://");

            for line in lines {
                let line = line.unwrap();
                if line.len() < 3 { break }
                let mut parts = line.as_slice().splitn(1, ':');
                headers.insert(parts.next().unwrap().to_string(),
                               parts.next().unwrap().slice_from(1).to_string());
            }
        }

        let mut handle = http::handle();
        let mut req = match method.as_slice() {
            "PUT" => handle.put(url, &mut socket),
            "POST" => handle.post(url, &mut socket),
            "DELETE" => handle.delete(url),
            "GET" => handle.get(url),
            _ => fail!("unknown method: {}", method),
        };
        for (k, v) in headers.iter() {
            let v = v.as_slice().trim();
            match k.as_slice() {
                "Content-Length" => req = req.content_length(from_str(v).unwrap()),
                "Content-Type" => req = req.content_type(v),
                "Transfer-Encoding" => {}
                k => req = req.header(k, v),
            }
        }
        req.exec().unwrap()
    }

    fn respond<W: Writer>(response: http::Response, mut socket: W) {
        socket.write_str(format!("HTTP/1.1 {}\r\n",
                                 response.get_code()).as_slice())
              .unwrap();
        for (k, v) in response.get_headers().iter() {
            if k.as_slice() == "transfer-encoding" { continue }
            for v in v.iter() {
                socket.write(k.as_bytes()).unwrap();
                socket.write(b": ").unwrap();
                socket.write(v.as_bytes()).unwrap();
                socket.write(b"\r\n").unwrap();
            }
        }
        socket.write(b"\r\n").unwrap();
        socket.write(response.get_body()).unwrap();
    }
}

fn replay_http(socket: TcpStream, data: &mut BufferedStream<File>) {
    let mut writer = socket.clone();
    let mut socket = BufferedReader::new(socket);

    let request = data.read_line().unwrap();
    let mut request = request.as_slice().split(' ');
    assert_eq!(request.next().unwrap().as_slice(), "===REQUEST");
    let request_size: uint = from_str(request.next().unwrap().trim()).unwrap();

    let expected = data.read_exact(request_size).unwrap();
    let mut expected_lines = expected.as_slice().split(|b| *b == b'\n')
                                     .map(|s| str::from_utf8(s).unwrap())
                                     .map(|s| format!("{}\n", s));
    let mut actual_lines = socket.lines().map(|s| s.unwrap());

    // validate the headers
    let mut expected: HashSet<String> = expected_lines.by_ref()
                                                      .take_while(|l| l.len() > 2)
                                                      .collect();
    let mut found = HashSet::new();
    println!("expecting: {}", expected);
    for line in actual_lines.by_ref().take_while(|l| l.len() > 2) {
        println!("received: {}", line.as_slice().trim());
        if !found.insert(line.clone()) { continue }
        if expected.remove(&line) { continue }
        if line.as_slice().starts_with("Date:") { continue }
        if line.as_slice().starts_with("Authorization:") { continue }
        fail!("unexpected header: {}", line);
    }
    for line in expected.iter() {
        if line.as_slice().starts_with("Date:") { continue }
        if line.as_slice().starts_with("Authorization:") { continue }
        fail!("didn't receive header: {}", line);
    }

    // TODO: validate the body

    data.read_line().unwrap();
    let response = data.read_line().unwrap();
    let mut response = response.as_slice().split(' ');
    assert_eq!(response.next().unwrap().as_slice(), "===RESPONSE");
    let response_size: uint = from_str(response.next().unwrap().trim()).unwrap();
    let response = data.read_exact(response_size).unwrap();
    let mut lines = response.as_slice().split(|b| *b == b'\n')
                            .map(|s| str::from_utf8(s).unwrap());
    for line in lines {
        if line.starts_with("Date:") { continue }
        writer.write_str(line).unwrap();
        writer.write(b"\r\n").unwrap();
    }
    data.read_line().unwrap();
}
