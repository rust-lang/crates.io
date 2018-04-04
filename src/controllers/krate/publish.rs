//! Functionality related to publishing a new crate or version of a crate.

use std::cmp;
use std::collections::HashMap;
use std::sync::Arc;

use hex::ToHex;
use serde_json;

use git;
use render;
use util::{read_fill, read_le_u32};
use util::{internal, ChainError};

use controllers::prelude::*;
use views::{EncodableCrate, EncodableCrateUpload};
use models::{Badge, Category, Keyword, NewCrate, NewVersion, Rights, User};
use models::dependency;

/// Handles the `PUT /crates/new` route.
/// Used by `cargo publish` to publish a new crate or to publish a new version of an
/// existing crate.
///
/// Currently blocks the HTTP thread, perhaps some function calls can spawn new
/// threads and return completion or error through other methods  a `cargo publish
/// --status` command, via crates.io's front end, or email.
pub fn publish(req: &mut Request) -> CargoResult<Response> {
    let app = Arc::clone(req.app());
    let (new_crate, user) = parse_new_headers(req)?;

    let name = &*new_crate.name;
    let vers = &*new_crate.vers;
    let links = new_crate.links.clone();
    let repo = new_crate.repository.as_ref().map(|s| &**s);
    let features = new_crate
        .features
        .iter()
        .map(|(k, v)| {
            (
                k[..].to_string(),
                v.iter().map(|v| v[..].to_string()).collect(),
            )
        })
        .collect::<HashMap<String, Vec<String>>>();
    let keywords = new_crate
        .keywords
        .as_ref()
        .map(|kws| kws.iter().map(|kw| &***kw).collect())
        .unwrap_or_else(Vec::new);

    let categories = new_crate.categories.as_ref().map(|s| &s[..]).unwrap_or(&[]);
    let categories: Vec<_> = categories.iter().map(|k| &***k).collect();

    let conn = req.db_conn()?;
    // Create a transaction on the database, if there are no errors,
    // commit the transactions to record a new or updated crate.
    conn.transaction(|| {
        // Persist the new crate, if it doesn't already exist
        let persist = NewCrate {
            name,
            description: new_crate.description.as_ref().map(|s| &**s),
            homepage: new_crate.homepage.as_ref().map(|s| &**s),
            documentation: new_crate.documentation.as_ref().map(|s| &**s),
            readme: new_crate.readme.as_ref().map(|s| &**s),
            readme_file: new_crate.readme_file.as_ref().map(|s| &**s),
            repository: repo,
            license: new_crate.license.as_ref().map(|s| &**s),
            max_upload_size: None,
        };

        let license_file = new_crate.license_file.as_ref().map(|s| &**s);
        let krate = persist.create_or_update(&conn, license_file, user.id)?;

        let owners = krate.owners(&conn)?;
        if user.rights(req.app(), &owners)? < Rights::Publish {
            return Err(human(
                "this crate exists but you don't seem to be an owner. \
                 If you believe this is a mistake, perhaps you need \
                 to accept an invitation to be an owner before \
                 publishing.",
            ));
        }

        if &krate.name != name {
            return Err(human(&format_args!(
                "crate was previously named `{}`",
                krate.name
            )));
        }

        let length = req.content_length()
            .chain_error(|| human("missing header: Content-Length"))?;
        let max = krate
            .max_upload_size
            .map(|m| m as u64)
            .unwrap_or(app.config.max_upload_size);
        if length > max {
            return Err(human(&format_args!("max upload size is: {}", max)));
        }

        // This is only redundant for now. Eventually the duplication will be removed.
        let license = new_crate.license.clone();

        // Persist the new version of this crate
        let version = NewVersion::new(krate.id, vers, &features, license, license_file)?
            .save(&conn, &new_crate.authors)?;

        // Link this new version to all dependencies
        let git_deps = dependency::add_dependencies(&conn, &new_crate.deps, version.id)?;

        // Update all keywords for this crate
        Keyword::update_crate(&conn, &krate, &keywords)?;

        // Update all categories for this crate, collecting any invalid categories
        // in order to be able to warn about them
        let ignored_invalid_categories = Category::update_crate(&conn, &krate, &categories)?;

        // Update all badges for this crate, collecting any invalid badges in
        // order to be able to warn about them
        let ignored_invalid_badges = Badge::update_crate(&conn, &krate, new_crate.badges.as_ref())?;
        let max_version = krate.max_version(&conn)?;

        // Render the README for this crate
        let readme = match new_crate.readme.as_ref() {
            Some(readme) => Some(render::readme_to_html(
                &**readme,
                new_crate.readme_file.as_ref().map_or("README.md", |s| &**s),
                repo,
            )?),
            None => None,
        };

        // Upload the crate, return way to delete the crate from the server
        // If the git commands fail below, we shouldn't keep the crate on the
        // server.
        let max_unpack = cmp::max(app.config.max_unpack_size, max);
        let (cksum, mut crate_bomb, mut readme_bomb) =
            app.config
                .uploader
                .upload_crate(req, &krate, readme, max, max_unpack, vers)?;
        version.record_readme_rendering(&conn)?;

        let mut hex_cksum = String::new();
        cksum.write_hex(&mut hex_cksum)?;

        // Register this crate in our local git repo.
        let git_crate = git::Crate {
            name: name.to_string(),
            vers: vers.to_string(),
            cksum: hex_cksum,
            features,
            deps: git_deps,
            yanked: Some(false),
            links,
        };
        git::add_crate(&**req.app(), &git_crate).chain_error(|| {
            internal(&format_args!(
                "could not add crate `{}` to the git repo",
                name
            ))
        })?;

        // Now that we've come this far, we're committed!
        crate_bomb.path = None;
        readme_bomb.path = None;

        #[derive(Serialize)]
        struct Warnings<'a> {
            invalid_categories: Vec<&'a str>,
            invalid_badges: Vec<&'a str>,
        }
        let warnings = Warnings {
            invalid_categories: ignored_invalid_categories,
            invalid_badges: ignored_invalid_badges,
        };

        #[derive(Serialize)]
        struct R<'a> {
            #[serde(rename = "crate")]
            krate: EncodableCrate,
            warnings: Warnings<'a>,
        }
        Ok(req.json(&R {
            krate: krate.minimal_encodable(&max_version, None, false, None),
            warnings,
        }))
    })
}

/// Used by the `krate::new` function.
///
/// This function parses the JSON headers to interpret the data and validates
/// the data during and after the parsing. Returns crate metadata and user
/// information.
fn parse_new_headers(req: &mut Request) -> CargoResult<(EncodableCrateUpload, User)> {
    // Read the json upload request
    let amt = u64::from(read_le_u32(req.body())?);
    let max = req.app().config.max_upload_size;
    if amt > max {
        return Err(human(&format_args!("max upload size is: {}", max)));
    }
    let mut json = vec![0; amt as usize];
    read_fill(req.body(), &mut json)?;
    let json = String::from_utf8(json).map_err(|_| human("json body was not valid utf-8"))?;
    let new: EncodableCrateUpload = serde_json::from_str(&json)
        .map_err(|e| human(&format_args!("invalid upload request: {}", e)))?;

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool {
        s.map_or(true, |s| s.is_empty())
    }
    let mut missing = Vec::new();

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if new.authors.iter().all(|s| s.is_empty()) {
        missing.push("authors");
    }
    if !missing.is_empty() {
        return Err(human(&format_args!(
            "missing or empty metadata fields: {}. Please \
             see https://doc.rust-lang.org/cargo/reference/manifest.html for \
             how to upload metadata",
            missing.join(", ")
        )));
    }

    let user = req.user()?;
    Ok((new, user.clone()))
}
