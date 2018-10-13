extern crate base64;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;

use std::collections::HashSet;
use std::fs::{self, File};
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

#[derive(Serialize, Deserialize)]
struct Exchange<T> {
    request: Request<T>,
    response: Response<T>,
}

#[derive(Serialize, Deserialize)]
struct Request<T> {
    uri: String,
    method: String,
    headers: HashSet<(String, String)>,
    body: T,
}

#[derive(Serialize, Deserialize)]
struct Response<T> {
    status: u16,
    headers: HashSet<(String, String)>,
    body: T,
}

impl From<Exchange<Vec<u8>>> for Exchange<String> {
    fn from(from: Exchange<Vec<u8>>) -> Self {
        let request = from.request;
        let request = Request {
            uri: request.uri,
            method: request.method,
            headers: request.headers,
            body: base64::encode(&request.body),
        };
        let response = from.response;
        let response = Response {
            status: response.status,
            headers: response.headers,
            body: base64::encode(&response.body),
        };
        Exchange { request, response }
    }
}

fn read(path: &PathBuf) -> io::Result<Vec<Exchange<Vec<u8>>>> {
    let mut file = File::open(path)?;
    let mut json = String::new();
    file.read_to_string(&mut json)?;
    Ok(serde_json::from_str(&json)?)
}

fn write(path: &PathBuf, data: &[Exchange<String>]) -> io::Result<()> {
    let json = serde_json::to_string_pretty(&data)?;
    let mut file = File::create(path)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}

fn main() -> io::Result<()> {
    for file in fs::read_dir("./src/tests/http-data")? {
        let file = file?;
        let path = file.path();
        if path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("gh-crates-test")
        {
            continue;
        };
        let data = read(&path)?;
        let data: Vec<_> = data.into_iter().map(Into::into).collect();
        write(&path, &data)?
    }
    Ok(())
}
