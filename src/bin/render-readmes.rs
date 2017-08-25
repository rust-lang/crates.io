// Iterates over every crate versions ever uploaded and (re-)renders their
// readme using the Markdown renderer from the cargo_registry crate.
//
// Warning: this can take a lot of time.

#![deny(warnings)]

#[macro_use]
extern crate serde_derive;

extern crate cargo_registry;
extern crate curl;
extern crate diesel;
extern crate docopt;
extern crate flate2;
extern crate s3;
extern crate tar;
extern crate time;
extern crate toml;
extern crate url;

use curl::easy::{Easy, List};
use diesel::prelude::*;
use docopt::Docopt;
use flate2::read::GzDecoder;
use std::io::{Cursor, Read};
use std::path::Path;
use std::thread;
use tar::Archive;
use url::Url;

use cargo_registry::{Config, Version};
use cargo_registry::schema::*;
use cargo_registry::render::markdown_to_html;

const DEFAULT_PAGE_SIZE: i64 = 25;
const USAGE: &'static str = "
Usage: render-readmes [options]
       render-readmes --help

Options:
    -h, --help         Show this message.
    --page-size NUM    How many versions should be queried and processed at a time.
";

#[derive(Deserialize)]
struct Args {
    flag_page_size: Option<i64>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let config: Config = Default::default();
    let conn = cargo_registry::db::connect_now().unwrap();
    let versions_count = versions::table.count().get_result::<i64>(&conn).expect(
        "error counting versions",
    );

    let page_size = args.flag_page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    let pages = if versions_count % page_size == 0 {
        versions_count / page_size
    } else {
        versions_count / page_size + 1
    };
    for current_page in 0..pages {
        let versions: Vec<(Version, String)> = versions::table
            .inner_join(crates::table)
            .select((versions::all_columns, crates::name))
            .limit(page_size)
            .offset(current_page * page_size)
            .load::<(Version, String)>(&conn)
            .expect("error loading versions");
        let mut tasks = Vec::with_capacity(page_size as usize);
        for (version, krate_name) in versions {
            let config = config.clone();
            version.record_readme_rendering(&conn).expect(&format!(
                "[{}-{}] Couldn't record rendering time",
                krate_name,
                version.num
            ));
            let handle = thread::spawn(move || {
                println!("[{}-{}] Rendering README...", krate_name, version.num);
                let readme = get_readme(&config, &version, &krate_name);
                if readme.is_none() {
                    return;
                }
                let readme = readme.unwrap();
                let readme_path =
                    format!("readmes/{}/{}-{}.html", krate_name, krate_name, version.num);
                let readme_len = readme.len();
                let mut body = Cursor::new(readme.into_bytes());
                config
                    .uploader
                    .upload(
                        Easy::new(),
                        &readme_path,
                        &mut body,
                        "text/html",
                        readme_len as u64,
                    )
                    .expect(&format!(
                        "[{}-{}] Couldn't upload file to S3",
                        krate_name,
                        version.num
                    ));
            });
            tasks.push(handle);
        }
        for handle in tasks {
            if let Err(err) = handle.join() {
                println!("Thread panicked: {:?}", err);
            }
        }
    }
}

/// Renders the readme of an uploaded crate version.
fn get_readme(config: &Config, version: &Version, krate_name: &str) -> Option<String> {
    let mut handle = Easy::new();
    let location = match config.uploader.crate_location(
        &krate_name,
        &version.num.to_string(),
    ) {
        Some(l) => l,
        None => return None,
    };
    let date = time::now().rfc822z().to_string();
    let url = Url::parse(&location).expect(&format!(
        "[{}-{}] Couldn't parse crate URL",
        krate_name,
        version.num
    ));

    let mut headers = List::new();
    headers
        .append(&format!("Host: {}", url.host().unwrap()))
        .unwrap();
    headers.append(&format!("Date: {}", date)).unwrap();

    handle.url(url.as_str()).unwrap();
    handle.get(true).unwrap();
    handle.http_headers(headers).unwrap();

    let mut response = Vec::new();
    {
        let mut req = handle.transfer();
        req.write_function(|data| {
            response.extend(data);
            Ok(data.len())
        }).unwrap();
        if let Err(err) = req.perform() {
            println!(
                "[{}-{}] Unable to fetch crate: {}",
                krate_name,
                version.num,
                err
            );
            return None;
        }
    }
    if handle.response_code().unwrap() != 200 {
        let response = String::from_utf8_lossy(&response);
        println!(
            "[{}-{}] Failed to get a 200 response: {}",
            krate_name,
            version.num,
            response
        );
        return None;
    }
    let reader = Cursor::new(response);
    let reader = GzDecoder::new(reader).expect(&format!(
        "[{}-{}] Invalid gzip header",
        krate_name,
        version.num
    ));
    let mut archive = Archive::new(reader);
    let mut entries = archive.entries().expect(&format!(
        "[{}-{}] Invalid tar archive entries",
        krate_name,
        version.num
    ));
    let manifest: Manifest = {
        let path = format!("{}-{}/Cargo.toml", krate_name, version.num);
        let contents = find_file_by_path(&mut entries, Path::new(&path), &version, &krate_name);
        toml::from_str(&contents).expect(&format!(
            "[{}-{}] Syntax error in manifest file",
            krate_name,
            version.num
        ))
    };
    if manifest.package.readme.is_none() {
        return None;
    }
    let rendered = {
        let path = format!(
            "{}-{}/{}",
            krate_name,
            version.num,
            manifest.package.readme.unwrap()
        );
        let contents = find_file_by_path(&mut entries, Path::new(&path), &version, &krate_name);
        markdown_to_html(&contents).expect(&format!(
            "[{}-{}] Couldn't render README",
            krate_name,
            version.num
        ))
    };
    return Some(rendered);
    #[derive(Deserialize)]
    struct Package {
        readme: Option<String>,
    }
    #[derive(Deserialize)]
    struct Manifest {
        package: Package,
    }
}

/// Search an entry by its path in a Tar archive.
fn find_file_by_path<R: Read>(
    entries: &mut tar::Entries<R>,
    path: &Path,
    version: &Version,
    krate_name: &str,
) -> String {
    let mut file = entries
        .find(|entry| match *entry {
            Err(_) => false,
            Ok(ref file) => {
                let filepath = match file.path() {
                    Ok(p) => p,
                    Err(_) => return false,
                };
                return filepath == path;
            }
        })
        .expect(&format!(
            "[{}-{}] couldn't open file: {}",
            krate_name,
            version.num,
            path.display()
        ))
        .expect(&format!(
            "[{}-{}] file is not present: {}",
            krate_name,
            version.num,
            path.display()
        ));
    let mut contents = String::new();
    file.read_to_string(&mut contents).expect(&format!(
        "[{}-{}] Couldn't read file contents",
        krate_name,
        version.num
    ));
    contents
}
