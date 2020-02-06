//! Functionality related to publishing a new crate or version of a crate.

use hex::ToHex;
use std::sync::Arc;
use swirl::Job;

use crate::controllers::cargo_prelude::*;
use crate::git;
use crate::models::dependency;
use crate::models::{
    insert_version_owner_action, Badge, Category, Keyword, NewCrate, NewVersion, Rights,
    VersionAction,
};

use crate::render;
use crate::util::{read_fill, read_le_u32, Maximums};
use crate::views::{EncodableCrateUpload, GoodCrate, PublishWarnings};

/// Handles the `PUT /crates/new` route.
/// Used by `cargo publish` to publish a new crate or to publish a new version of an
/// existing crate.
///
/// Currently blocks the HTTP thread, perhaps some function calls can spawn new
/// threads and return completion or error through other methods  a `cargo publish
/// --status` command, via crates.io's front end, or email.
pub fn publish(req: &mut dyn Request) -> AppResult<Response> {
    let app = Arc::clone(req.app());

    // The format of the req.body() of a publish request is as follows:
    //
    // metadata length
    // metadata in JSON about the crate being published
    // .crate tarball length
    // .crate tarball file
    //
    // - The metadata is read and interpreted in the parse_new_headers function.
    // - The .crate tarball length is read in this function in order to save the size of the file
    //   in the version record in the database.
    // - Then the .crate tarball length is passed to the upload_crate function where the actual
    //   file is read and uploaded.

    let new_crate = parse_new_headers(req)?;

    let conn = app.primary_database.get()?;
    let ids = req.authenticate(&conn)?;
    let user = ids.find_user(&conn)?;

    let verified_email_address = user.verified_email(&conn)?;
    let verified_email_address = verified_email_address.ok_or_else(|| {
        cargo_err(
            "A verified email address is required to publish crates to crates.io. \
             Visit https://crates.io/me to set and verify your email address.",
        )
    })?;

    // Create a transaction on the database, if there are no errors,
    // commit the transactions to record a new or updated crate.
    conn.transaction(|| {
        let name = new_crate.name;
        let vers = &*new_crate.vers;
        let links = new_crate.links;
        let repo = new_crate.repository;
        let features = new_crate
            .features
            .into_iter()
            .map(|(k, v)| (k.0, v.into_iter().map(|v| v.0).collect()))
            .collect();
        let keywords = new_crate
            .keywords
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();
        let categories = new_crate
            .categories
            .iter()
            .map(|s| s.as_str())
            .collect::<Vec<_>>();

        // Persist the new crate, if it doesn't already exist
        let persist = NewCrate {
            name: &name,
            description: new_crate.description.as_deref(),
            homepage: new_crate.homepage.as_deref(),
            documentation: new_crate.documentation.as_deref(),
            readme: new_crate.readme.as_deref(),
            repository: repo.as_deref(),
            max_upload_size: None,
        };

        let license_file = new_crate.license_file.as_deref();
        let krate =
            persist.create_or_update(&conn, user.id, Some(&app.config.publish_rate_limit))?;

        let owners = krate.owners(&conn)?;
        if user.rights(req.app(), &owners)? < Rights::Publish {
            return Err(cargo_err(
                "this crate exists but you don't seem to be an owner. \
                 If you believe this is a mistake, perhaps you need \
                 to accept an invitation to be an owner before \
                 publishing.",
            ));
        }

        if krate.name != *name {
            return Err(cargo_err(&format_args!(
                "crate was previously named `{}`",
                krate.name
            )));
        }

        // Length of the .crate tarball, which appears after the metadata in the request body.
        // TODO: Not sure why we're using the total content length (metadata + .crate file length)
        // to compare against the max upload size... investigate that and perhaps change to use
        // this file length.
        let file_length = read_le_u32(req.body())?;

        let content_length = req
            .content_length()
            .chain_error(|| cargo_err("missing header: Content-Length"))?;

        let maximums = Maximums::new(
            krate.max_upload_size,
            app.config.max_upload_size,
            app.config.max_unpack_size,
        );

        if content_length > maximums.max_upload_size {
            return Err(cargo_err(&format_args!(
                "max upload size is: {}",
                maximums.max_upload_size
            )));
        }

        // This is only redundant for now. Eventually the duplication will be removed.
        let license = new_crate.license.clone();

        // Persist the new version of this crate
        let version = NewVersion::new(
            krate.id,
            vers,
            &features,
            license,
            license_file,
            // Downcast is okay because the file length must be less than the max upload size
            // to get here, and max upload sizes are way less than i32 max
            file_length as i32,
            user.id,
        )?
        .save(&conn, &new_crate.authors, &verified_email_address)?;

        insert_version_owner_action(
            &conn,
            version.id,
            user.id,
            ids.api_token_id(),
            VersionAction::Publish,
        )?;

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
        let top_versions = krate.top_versions(&conn)?;

        if let Some(readme) = new_crate.readme {
            render::render_and_upload_readme(
                version.id,
                readme,
                new_crate
                    .readme_file
                    .unwrap_or_else(|| String::from("README.md")),
                repo,
            )
            .enqueue(&conn)
            .map_err(|e| AppError::from_std_error(e))?;
        }

        let cksum = app
            .config
            .uploader
            .upload_crate(req, &krate, maximums, vers)?;

        let mut hex_cksum = String::new();
        cksum.write_hex(&mut hex_cksum)?;

        // Register this crate in our local git repo.
        let git_crate = git::Crate {
            name: name.0,
            vers: vers.to_string(),
            cksum: hex_cksum,
            features,
            deps: git_deps,
            yanked: Some(false),
            links,
        };
        git::add_crate(git_crate)
            .enqueue(&conn)
            .map_err(|e| AppError::from_std_error(e))?;

        // The `other` field on `PublishWarnings` was introduced to handle a temporary warning
        // that is no longer needed. As such, crates.io currently does not return any `other`
        // warnings at this time, but if we need to, the field is available.
        let warnings = PublishWarnings {
            invalid_categories: ignored_invalid_categories,
            invalid_badges: ignored_invalid_badges,
            other: vec![],
        };

        Ok(req.json(&GoodCrate {
            krate: krate.minimal_encodable(&top_versions, None, false, None),
            warnings,
        }))
    })
}

/// Used by the `krate::new` function.
///
/// This function parses the JSON headers to interpret the data and validates
/// the data during and after the parsing. Returns crate metadata.
fn parse_new_headers(req: &mut dyn Request) -> AppResult<EncodableCrateUpload> {
    // Read the json upload request
    let metadata_length = u64::from(read_le_u32(req.body())?);
    req.mut_extensions().insert(metadata_length);

    let max = req.app().config.max_upload_size;
    if metadata_length > max {
        return Err(cargo_err(&format_args!("max upload size is: {}", max)));
    }
    let mut json = vec![0; metadata_length as usize];
    read_fill(req.body(), &mut json)?;
    let json = String::from_utf8(json).map_err(|_| cargo_err("json body was not valid utf-8"))?;
    let new: EncodableCrateUpload = serde_json::from_str(&json)
        .map_err(|e| cargo_err(&format_args!("invalid upload request: {}", e)))?;

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool {
        s.map_or(true, String::is_empty)
    }
    let mut missing = Vec::new();

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if new.authors.iter().all(String::is_empty) {
        missing.push("authors");
    }
    if !missing.is_empty() {
        return Err(cargo_err(&format_args!(
            "missing or empty metadata fields: {}. Please \
             see https://doc.rust-lang.org/cargo/reference/manifest.html for \
             how to upload metadata",
            missing.join(", ")
        )));
    }

    Ok(new)
}
