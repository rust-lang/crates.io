use axum::extract::{ConnectInfo, Path, Request};
use axum::response::{IntoResponse, Response};
use http::header::HeaderName;
use http::request::Parts;
use http::{HeaderMap, HeaderValue, StatusCode, header};
use hyper::body::Buf;
use std::io::{BufRead, Read};
use std::net::SocketAddr;
use std::process::{Command, Stdio};

pub async fn http_backend(
    Path(path): Path<String>,
    ConnectInfo(remote_addr): ConnectInfo<SocketAddr>,
    req: Request,
) -> Result<Response, StatusCode> {
    let path = format!("/{path}");

    let (req, body) = req.into_parts();
    let body = axum::body::to_bytes(body, usize::MAX)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut cmd = Command::new("git");
    cmd.arg("http-backend");

    // Required environment variables
    cmd.env("REQUEST_METHOD", req.method.as_str());
    cmd.env("GIT_PROJECT_ROOT", "./tmp/index-bare");
    cmd.env("PATH_INFO", &path);
    cmd.env("REMOTE_USER", "");
    cmd.env("REMOTE_ADDR", remote_addr.to_string());
    cmd.env("QUERY_STRING", req.uri.query().unwrap_or_default());
    cmd.env("CONTENT_TYPE", header(&req, header::CONTENT_TYPE));

    cmd.stderr(Stdio::inherit())
        .stdout(Stdio::piped())
        .stdin(Stdio::piped());

    let mut p = cmd.spawn().map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tokio::task::spawn_blocking(move || {
        // Pass in the body of the request (if any)
        let mut body_reader = body.reader();
        std::io::copy(&mut body_reader, &mut p.stdin.take().unwrap())
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Parse the headers coming out, and the pass through the rest of the
        // process back down the stack.
        //
        // Note that we have to be careful to not drop the process which will wait
        // for the process to exit (and we haven't read stdout)
        let mut rdr = std::io::BufReader::new(p.stdout.take().unwrap());

        let mut headers = HeaderMap::new();
        for line in rdr.by_ref().lines() {
            let line = line.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            if line.is_empty() || line == "\r" {
                break;
            }

            let (key, value) = line.split_once(':').unwrap();
            let value = &value[1..];
            headers.insert(
                key.parse::<HeaderName>().unwrap(),
                HeaderValue::from_str(value).unwrap(),
            );
        }

        let status_code = headers
            .remove("Status")
            .as_ref()
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.split(' ').next())
            .unwrap_or_default()
            .parse()
            .unwrap_or(StatusCode::OK);

        let mut body = Vec::new();
        rdr.read_to_end(&mut body)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        Ok((status_code, headers, body).into_response())
    })
    .await
    .unwrap_or_else(|_| Err(StatusCode::INTERNAL_SERVER_ERROR))
}

/// Obtain the value of a header
///
/// If multiple headers have the same name, only one will be returned.
///
/// If there is no header, of if there is an error parsings it as utf8
/// then an empty slice will be returned.
fn header(req: &Parts, name: HeaderName) -> &str {
    req.headers
        .get(name)
        .map(|value| value.to_str().unwrap_or_default())
        .unwrap_or_default()
}
