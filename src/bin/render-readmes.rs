// Iterates over every crate versions ever uploaded and (re-)renders their
// readme using the Markdown renderer from the cargo_registry crate.
//
// Warning: this can take a lot of time.

#![deny(warnings)]

#[macro_use]
extern crate serde_derive;

extern crate cargo_registry;
extern crate chrono;
extern crate curl;
extern crate diesel;
extern crate docopt;
extern crate flate2;
extern crate itertools;
extern crate tar;
extern crate toml;
extern crate url;

use curl::easy::{Easy, List};
use chrono::{TimeZone, Utc};
use diesel::prelude::*;
use diesel::expression::any;
use docopt::Docopt;
use flate2::read::GzDecoder;
use itertools::Itertools;
use std::io::{Cursor, Read};
use std::path::Path;
use std::thread;
use tar::Archive;
use url::Url;

use cargo_registry::{Config, Version};
use cargo_registry::schema::*;
use cargo_registry::render::markdown_to_html;

const DEFAULT_PAGE_SIZE: usize = 25;
const USAGE: &'static str = "
Usage: render-readmes [options]
       render-readmes --help

Options:
    -h, --help         Show this message.
    --page-size NUM    How many versions should be queried and processed at a time.
    --older-than DATE  Only rerender readmes that are older than this date.
    --crate NAME       Only rerender readmes for the specified crate.
";

#[derive(Deserialize)]
struct Args {
    flag_page_size: Option<usize>,
    flag_older_than: Option<String>,
    flag_crate: Option<String>,
}

fn main() {
    let args: Args = Docopt::new(USAGE)
        .and_then(|d| d.deserialize())
        .unwrap_or_else(|e| e.exit());
    let config: Config = Default::default();
    let conn = cargo_registry::db::connect_now().unwrap();

    let start_time = Utc::now();

    let older_than = if let Some(ref time) = args.flag_older_than {
        Utc.datetime_from_str(&time, "%Y-%m-%d %H:%M:%S")
            .expect("Could not parse --older-than argument as a time")
    } else {
        start_time
    };
    let older_than = older_than.naive_utc();

    println!("Start time:                   {}", start_time);
    println!("Rendering readmes older than: {}", older_than);

    let mut query = versions::table
        .inner_join(crates::table)
        .filter(
            versions::readme_rendered_at
                .lt(older_than)
                .or(versions::readme_rendered_at.is_null()),
        )
        .select(versions::id)
        .into_boxed();

    if let Some(crate_name) = args.flag_crate {
        println!("Rendering readmes for {}", crate_name);
        query = query.filter(crates::name.eq(crate_name));
    }

    let version_ids = query
        .load::<(i32)>(&conn)
        .expect("error loading version ids");

    let total_versions = version_ids.len();
    println!("Rendering {} versions", total_versions);

    let page_size = args.flag_page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    let total_pages = total_versions / page_size;
    let total_pages = if total_versions % page_size == 0 {
        total_pages
    } else {
        total_pages + 1
    };

    for (page_num, version_ids_chunk) in version_ids
        .into_iter()
        .chunks(page_size)
        .into_iter()
        .enumerate()
    {
        println!(
            "= Page {} of {} ==================================",
            page_num + 1,
            total_pages
        );

        let ids: Vec<_> = version_ids_chunk.collect();

        let versions = versions::table
            .inner_join(crates::table)
            .filter(versions::id.eq(any(ids)))
            .select((versions::all_columns, crates::name))
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
                let readme_path = format!("readmes/{0}/{0}-{1}.html", krate_name, version.num);
                let readme_len = readme.len();
                config
                    .uploader
                    .upload(
                        Easy::new(),
                        &readme_path,
                        readme.as_bytes(),
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
    let location = match config
        .uploader
        .crate_location(&krate_name, &version.num.to_string())
    {
        Some(l) => l,
        None => return None,
    };
    let date = Utc::now().to_rfc2822();
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
