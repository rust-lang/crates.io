use std::io;
use std::io::{Command, BufferedReader, Process, IoResult, File, fs};
use std::ascii::StrAsciiExt;
use std::collections::HashMap;
use std::io::util;

use conduit::{Request, Response};

use app::{App, RequestApp};
use util::{CargoResult, exec};
use package::Package;

pub fn serve_index(req: &mut Request) -> CargoResult<Response> {
    let mut cmd = Command::new("git");
    cmd.arg("http-backend");

    // Required environment variables
    cmd.env("REQUEST_METHOD",
            req.method().to_string().as_slice().to_ascii_upper());
    cmd.env("GIT_PROJECT_ROOT", &req.app().git_repo_bare);
    cmd.env("PATH_INFO", req.path().replace("/git/index", ""));
    cmd.env("REMOTE_USER", "");
    cmd.env("REMOTE_ADDR", req.remote_ip().to_string());
    cmd.env("QUERY_STRING", req.query_string().unwrap_or(""));
    cmd.env("CONTENT_TYPE", header(req, "Content-Type"));
    cmd.stderr(::std::io::process::InheritFd(2));
    let mut p = try!(cmd.spawn());

    // Pass in the body of the request (if any)
    //
    // As part of the CGI interface we're required to take care of gzip'd
    // requests. I'm not totally sure that this sequential copy is the best
    // thing to do or actually correct...
    if header(req, "Content-Encoding") == "gzip" {
        let mut gunzip = try!(Command::new("gunzip").arg("-c").spawn());
        try!(util::copy(&mut req.body(), &mut gunzip.stdin.take().unwrap()));
        try!(util::copy(&mut gunzip.stdout.take().unwrap(),
                        &mut p.stdin.take().unwrap()));
    } else {
        try!(util::copy(&mut req.body(), &mut p.stdin.take().unwrap()));
    }

    // Parse the headers coming out, and the pass through the rest of the
    // process back down the stack.
    //
    // Note that we have to be careful to not drop the process which will wait
    // for the process to exit (and we haven't read stdout)
    let mut rdr = BufferedReader::new(p.stdout.take().unwrap());

    let mut headers = HashMap::new();
    for line in rdr.lines() {
        let line = try!(line);
        if line.as_slice() == "\r\n" { break }

        let mut parts = line.as_slice().splitn(':', 2);
        let key = parts.next().unwrap();
        let value = parts.next().unwrap();
        let value = value.slice(1, value.len() - 2);
        headers.find_or_insert(key.to_string(), Vec::new()).push(value.to_string());
    }

    let (status_code, status_desc) = {
        let line = headers.pop_equiv(&"Status").unwrap_or(Vec::new());
        let line = line.move_iter().next().unwrap_or(String::new());
        let mut parts = line.as_slice().splitn(' ', 1);
        (from_str(parts.next().unwrap_or("")).unwrap_or(200),
         match parts.next() {
             Some("Not Found") => "Not Found",
             _ => "Ok",
         })
    };

    struct ProcessAndBuffer<R> { _p: Process, buf: BufferedReader<R> }
    impl<R: Reader> Reader for ProcessAndBuffer<R> {
        fn read(&mut self, b: &mut [u8]) -> IoResult<uint> { self.buf.read(b) }
    }
    return Ok(Response {
        status: (status_code, status_desc),
        headers: headers,
        body: box ProcessAndBuffer { _p: p, buf: rdr },
    });

    fn header<'a>(req: &'a Request, name: &str) -> &'a str {
        let h = req.headers().find(name).unwrap_or(Vec::new());
        h.as_slice().get(0).map(|s| *s).unwrap_or("")
    }
}

pub fn add_package(app: &App, package: &Package) -> CargoResult<()> {
    let path = app.git_repo_checkout.lock();
    let path = &*path;
    let name = package.name.as_slice();
    let (c1, c2) = match name.len() {
        0 => unreachable!(),
        1 => (format!("{}X", name.slice_to(1)), format!("XX")),
        2 => (format!("{}", name.slice_to(2)), format!("XX")),
        3 => (format!("{}", name.slice_to(2)), format!("{}X", name.char_at(2))),
        _ => (name.slice_to(2).to_string(), name.slice(2, 4).to_string()),
    };

    let dst = path.join(c1).join(c2).join(name);
    try!(fs::mkdir_recursive(&dst.dir_path(), io::UserRWX));
    try!(File::create(&dst).write(package.name.as_bytes()));

    macro_rules! git( ($($e:expr),*) => ({
        try!(exec(Command::new("git").cwd(path)$(.arg($e))*))
    }))

    git!("add", dst);
    git!("commit", "-m", format!("Adding package `{}`", package.name));
    git!("push");

    Ok(())
}
