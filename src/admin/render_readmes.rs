use crate::{
    config, db,
    models::Version,
    schema::{crates, readme_renderings, versions},
    uploaders::Uploader,
};
use std::{io::Read, path::Path, sync::Arc, thread};

use cargo_registry_markdown::text_to_html;
use chrono::{TimeZone, Utc};
use diesel::{dsl::any, prelude::*};
use flate2::read::GzDecoder;
use reqwest::{blocking::Client, header};
use tar::{self, Archive};

const CACHE_CONTROL_README: &str = "public,max-age=604800";
const USER_AGENT: &str = "crates-admin";

#[derive(clap::Parser, Debug)]
#[clap(
    name = "render-readmes",
    about = "Iterates over every crate versions ever uploaded and (re-)renders their \
        readme using the readme renderer from the cargo_registry crate.",
    after_help = "Warning: this can take a lot of time."
)]
pub struct Opts {
    /// How many versions should be queried and processed at a time.
    #[clap(long, default_value = "25")]
    page_size: usize,

    /// Only rerender readmes that are older than this date.
    #[clap(long)]
    older_than: Option<String>,

    /// Only rerender readmes for the specified crate.
    #[clap(long = "crate")]
    crate_name: Option<String>,
}

pub fn run(opts: Opts) {
    let base_config = Arc::new(config::Base::from_environment());
    let conn = db::connect_now().unwrap();

    let start_time = Utc::now();

    let older_than = if let Some(ref time) = opts.older_than {
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

    if let Some(crate_name) = opts.crate_name {
        println!("Rendering readmes for {}", crate_name);
        query = query.filter(crates::name.eq(crate_name));
    }

    let version_ids: Vec<i32> = query.load(&conn).expect("error loading version ids");

    let total_versions = version_ids.len();
    println!("Rendering {} versions", total_versions);

    let page_size = opts.page_size;

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

        let versions: Vec<(Version, String)> = versions::table
            .inner_join(crates::table)
            .filter(versions::id.eq(any(version_ids_chunk)))
            .select((versions::all_columns, crates::name))
            .load(&conn)
            .expect("error loading versions");

        let mut tasks = Vec::with_capacity(page_size as usize);
        for (version, krate_name) in versions {
            Version::record_readme_rendering(version.id, &conn).unwrap_or_else(|_| {
                panic!(
                    "[{}-{}] Couldn't record rendering time",
                    krate_name, version.num
                )
            });
            let client = client.clone();
            let base_config = base_config.clone();
            let handle = thread::spawn(move || {
                println!("[{}-{}] Rendering README...", krate_name, version.num);
                let readme = get_readme(base_config.uploader(), &client, &version, &krate_name);
                if readme.is_none() {
                    return;
                }
                let readme = readme.unwrap();
                let content_length = readme.len() as u64;
                let content = std::io::Cursor::new(readme);
                let readme_path = format!("readmes/{0}/{0}-{1}.html", krate_name, version.num);
                let mut extra_headers = header::HeaderMap::new();
                extra_headers.insert(
                    header::CACHE_CONTROL,
                    header::HeaderValue::from_static(CACHE_CONTROL_README),
                );
                base_config
                    .uploader()
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
    uploader: &Uploader,
    client: &Client,
    version: &Version,
    krate_name: &str,
) -> Option<String> {
    let pkg_name = format!("{}-{}", krate_name, version.num);

    let location = uploader.crate_location(krate_name, &version.num.to_string());

    let location = match uploader {
        Uploader::S3 { .. } => location,
        Uploader::Local => format!("http://localhost:8888/{}", location),
    };

    let mut extra_headers = header::HeaderMap::new();
    extra_headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(USER_AGENT),
    );
    let response = match client.get(&location).headers(extra_headers).send() {
        Ok(r) => r,
        Err(err) => {
            println!("[{}] Unable to fetch crate: {}", pkg_name, err);
            return None;
        }
    };

    if !response.status().is_success() {
        println!(
            "[{}] Failed to get a 200 response: {}",
            pkg_name,
            response.text().unwrap()
        );
        return None;
    }

    let reader = GzDecoder::new(response);
    let archive = Archive::new(reader);
    render_pkg_readme(archive, &pkg_name)
}

fn render_pkg_readme<R: Read>(mut archive: Archive<R>, pkg_name: &str) -> Option<String> {
    let mut entries = archive
        .entries()
        .unwrap_or_else(|_| panic!("[{}] Invalid tar archive entries", pkg_name));

    let manifest: Manifest = {
        let path = format!("{}/Cargo.toml", pkg_name);
        let contents = find_file_by_path(&mut entries, Path::new(&path), pkg_name)
            .unwrap_or_else(|| panic!("[{}] couldn't open file: Cargo.toml", pkg_name));
        toml::from_str(&contents)
            .unwrap_or_else(|_| panic!("[{}] Syntax error in manifest file", pkg_name))
    };

    let rendered = {
        let readme_path = manifest
            .package
            .readme
            .clone()
            .unwrap_or_else(|| "README.md".into());
        let path = format!("{}/{}", pkg_name, readme_path);
        let contents = find_file_by_path(&mut entries, Path::new(&path), pkg_name)?;
        text_to_html(
            &contents,
            &readme_path,
            manifest.package.repository.as_deref(),
        )
    };
    return Some(rendered);

    #[derive(Debug, Deserialize)]
    struct Package {
        readme: Option<String>,
        repository: Option<String>,
    }

    #[derive(Debug, Deserialize)]
    struct Manifest {
        package: Package,
    }
}

/// Search an entry by its path in a Tar archive.
fn find_file_by_path<R: Read>(
    entries: &mut tar::Entries<'_, R>,
    path: &Path,
    pkg_name: &str,
) -> Option<String> {
    let mut file = entries.filter_map(|entry| entry.ok()).find(|file| {
        let filepath = match file.path() {
            Ok(p) => p,
            Err(_) => return false,
        };
        filepath == path
    })?;

    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .unwrap_or_else(|_| panic!("[{}] Couldn't read file contents", pkg_name));
    Some(contents)
}

#[cfg(test)]
pub mod tests {
    use std::io::Write;
    use tar;

    use super::render_pkg_readme;

    pub fn add_file<W: Write>(pkg: &mut tar::Builder<W>, path: &str, content: &[u8]) {
        let mut header = tar::Header::new_gnu();
        header.set_size(content.len() as u64);
        header.set_cksum();
        pkg.append_data(&mut header, path, content).unwrap();
    }

    #[test]
    fn test_render_pkg_readme() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(
            &mut pkg,
            "foo-0.0.1/Cargo.toml",
            br#"
[package]
readme = "README.md"
"#,
        );
        add_file(&mut pkg, "foo-0.0.1/README.md", b"readme");
        let serialized_archive = pkg.into_inner().unwrap();
        let result =
            render_pkg_readme(tar::Archive::new(&*serialized_archive), "foo-0.0.1").unwrap();
        assert!(result.contains("readme"))
    }

    #[test]
    fn test_render_pkg_no_readme() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(
            &mut pkg,
            "foo-0.0.1/Cargo.toml",
            br#"
[package]
"#,
        );
        let serialized_archive = pkg.into_inner().unwrap();
        assert!(render_pkg_readme(tar::Archive::new(&*serialized_archive), "foo-0.0.1").is_none());
    }

    #[test]
    fn test_render_pkg_implicit_readme() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(
            &mut pkg,
            "foo-0.0.1/Cargo.toml",
            br#"
[package]
"#,
        );
        add_file(&mut pkg, "foo-0.0.1/README.md", b"readme");
        let serialized_archive = pkg.into_inner().unwrap();
        let result =
            render_pkg_readme(tar::Archive::new(&*serialized_archive), "foo-0.0.1").unwrap();
        assert!(result.contains("readme"))
    }

    #[test]
    fn test_render_pkg_readme_w_link() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(
            &mut pkg,
            "foo-0.0.1/Cargo.toml",
            br#"
[package]
readme = "README.md"
repository = "https://github.com/foo/foo"
"#,
        );
        add_file(
            &mut pkg,
            "foo-0.0.1/README.md",
            b"readme [link](./Other.md)",
        );
        let serialized_archive = pkg.into_inner().unwrap();
        let result =
            render_pkg_readme(tar::Archive::new(&*serialized_archive), "foo-0.0.1").unwrap();
        assert!(result.contains("\"https://github.com/foo/foo/blob/HEAD/./Other.md\""))
    }

    #[test]
    fn test_render_pkg_readme_not_at_root() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(
            &mut pkg,
            "foo-0.0.1/Cargo.toml",
            br#"
[package]
readme = "docs/README.md"
repository = "https://github.com/foo/foo"
"#,
        );
        add_file(
            &mut pkg,
            "foo-0.0.1/docs/README.md",
            b"docs/readme [link](./Other.md)",
        );
        let serialized_archive = pkg.into_inner().unwrap();
        let result =
            render_pkg_readme(tar::Archive::new(&*serialized_archive), "foo-0.0.1").unwrap();
        assert!(result.contains("docs/readme"));
        assert!(result.contains("\"https://github.com/foo/foo/blob/HEAD/docs/./Other.md\""))
    }
}
