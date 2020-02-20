// Iterates over every crate versions ever uploaded and (re-)renders their
// readme using the readme renderer from the cargo_registry crate.
//
// Warning: this can take a lot of time.

#![warn(clippy::all, rust_2018_idioms)]

#[macro_use]
extern crate serde;

use cargo_registry::{
    db,
    models::Version,
    render::readme_to_html,
    schema::{crates, readme_renderings, versions},
    Config,
};
use std::{io::Read, path::Path, thread};

use chrono::{TimeZone, Utc};
use diesel::{dsl::any, prelude::*};
use docopt::Docopt;
use flate2::read::GzDecoder;
use reqwest::{blocking::Client, header};
use tar::{self, Archive};

const CACHE_CONTROL_README: &str = "public,max-age=604800";
const DEFAULT_PAGE_SIZE: usize = 25;
const USAGE: &str = "
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
    let config = Config::default();
    let conn = db::connect_now().unwrap();

    let start_time = Utc::now();

    let older_than = if let Some(ref time) = args.flag_older_than {
        Utc.datetime_from_str(time, "%Y-%m-%d %H:%M:%S")
            .expect("Could not parse --older-than argument as a time")
    } else {
        start_time
    };
    let older_than = older_than.naive_utc();

    println!("Start time:                   {}", start_time);
    println!("Rendering readmes older than: {}", older_than);

    let mut query = versions::table
        .inner_join(crates::table)
        .left_outer_join(readme_renderings::table)
        .filter(
            readme_renderings::rendered_at
                .lt(older_than)
                .or(readme_renderings::version_id.is_null()),
        )
        .select(versions::id)
        .into_boxed();

    if let Some(crate_name) = args.flag_crate {
        println!("Rendering readmes for {}", crate_name);
        query = query.filter(crates::name.eq(crate_name));
    }

    let version_ids = query.load::<i32>(&conn).expect("error loading version ids");

    let total_versions = version_ids.len();
    println!("Rendering {} versions", total_versions);

    let page_size = args.flag_page_size.unwrap_or(DEFAULT_PAGE_SIZE);

    let total_pages = total_versions / page_size;
    let total_pages = if total_versions % page_size == 0 {
        total_pages
    } else {
        total_pages + 1
    };

    let client = Client::new();

    for (page_num, version_ids_chunk) in version_ids.chunks(page_size).enumerate() {
        println!(
            "= Page {} of {} ==================================",
            page_num + 1,
            total_pages
        );

        let versions = versions::table
            .inner_join(crates::table)
            .filter(versions::id.eq(any(version_ids_chunk)))
            .select((versions::all_columns, crates::name))
            .load::<(Version, String)>(&conn)
            .expect("error loading versions");

        let mut tasks = Vec::with_capacity(page_size as usize);
        for (version, krate_name) in versions {
            let config = config.clone();
            Version::record_readme_rendering(version.id, &conn).unwrap_or_else(|_| {
                panic!(
                    "[{}-{}] Couldn't record rendering time",
                    krate_name, version.num
                )
            });
            let client = client.clone();
            let handle = thread::spawn(move || {
                println!("[{}-{}] Rendering README...", krate_name, version.num);
                let readme = get_readme(&config, &client, &version, &krate_name);
                if readme.is_none() {
                    return;
                }
                let readme = readme.unwrap();
                let content_length = readme.len() as u64;
                let content = std::io::Cursor::new(readme);
                let readme_path = format!("readmes/{0}/{0}-{1}.html", krate_name, version.num);
                let mut extra_headers = header::HeaderMap::new();
                extra_headers.insert(header::CACHE_CONTROL, CACHE_CONTROL_README.parse().unwrap());
                config
                    .uploader
                    .upload(
                        &client,
                        &readme_path,
                        content,
                        content_length,
                        "text/html",
                        extra_headers,
                    )
                    .unwrap_or_else(|_| {
                        panic!(
                            "[{}-{}] Couldn't upload file to S3",
                            krate_name, version.num
                        )
                    });
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
fn get_readme(
    config: &Config,
    client: &Client,
    version: &Version,
    krate_name: &str,
) -> Option<String> {
    let location = config
        .uploader
        .crate_location(krate_name, &version.num.to_string());

    let response = match client.get(&location).send() {
        Ok(r) => r,
        Err(err) => {
            println!(
                "[{}-{}] Unable to fetch crate: {}",
                krate_name, version.num, err
            );
            return None;
        }
    };

    if !response.status().is_success() {
        println!(
            "[{}-{}] Failed to get a 200 response: {}",
            krate_name,
            version.num,
            response.text().unwrap()
        );
        return None;
    }

    let reader = GzDecoder::new(response);
    let mut archive = Archive::new(reader);
    let mut entries = archive.entries().unwrap_or_else(|_| {
        panic!(
            "[{}-{}] Invalid tar archive entries",
            krate_name, version.num
        )
    });
    let manifest: Manifest = {
        let path = format!("{}-{}/Cargo.toml", krate_name, version.num);
        let contents = find_file_by_path(&mut entries, Path::new(&path), version, krate_name);
        toml::from_str(&contents).unwrap_or_else(|_| {
            panic!(
                "[{}-{}] Syntax error in manifest file",
                krate_name, version.num
            )
        })
    };

    let rendered = {
        let path = format!(
            "{}-{}/{}",
            krate_name, version.num, manifest.package.readme?
        );
        let contents = find_file_by_path(&mut entries, Path::new(&path), version, krate_name);
        readme_to_html(
            &contents,
            manifest
                .package
                .readme_file
                .as_ref()
                .map_or("README.md", |e| &**e),
            manifest.package.repository.as_deref(),
        )
    };
    return Some(rendered);

    #[derive(Deserialize)]
    struct Package {
        readme: Option<String>,
        readme_file: Option<String>,
        repository: Option<String>,
    }

    #[derive(Deserialize)]
    struct Manifest {
        package: Package,
    }
}

/// Search an entry by its path in a Tar archive.
fn find_file_by_path<R: Read>(
    entries: &mut tar::Entries<'_, R>,
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
                filepath == path
            }
        })
        .unwrap_or_else(|| {
            panic!(
                "[{}-{}] couldn't open file: {}",
                krate_name,
                version.num,
                path.display()
            )
        })
        .unwrap_or_else(|_| {
            panic!(
                "[{}-{}] file is not present: {}",
                krate_name,
                version.num,
                path.display()
            )
        });
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap_or_else(|_| {
        panic!(
            "[{}-{}] Couldn't read file contents",
            krate_name, version.num
        )
    });
    contents
}
