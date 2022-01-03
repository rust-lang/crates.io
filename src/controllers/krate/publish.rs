//! Functionality related to publishing a new crate or version of a crate.

use flate2::read::GzDecoder;
use hex::ToHex;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Read;
use std::path::Path;
use std::sync::Arc;
use swirl::Job;

use crate::controllers::cargo_prelude::*;
use crate::git;
use crate::models::{
    insert_version_owner_action, Badge, Category, Crate, DependencyKind, Keyword, NewCrate,
    NewVersion, Rights, VersionAction,
};
use crate::worker;

use crate::schema::*;
use crate::util::errors::{cargo_err, AppResult};
use crate::util::{read_fill, read_le_u32, CargoVcsInfo, LimitErrorReader, Maximums};
use crate::views::{
    EncodableCrate, EncodableCrateDependency, EncodableCrateUpload, GoodCrate, PublishWarnings,
};

pub const MISSING_RIGHTS_ERROR_MESSAGE: &str =
    "this crate exists but you don't seem to be an owner. \
     If you believe this is a mistake, perhaps you need \
     to accept an invitation to be an owner before \
     publishing.";

pub const WILDCARD_ERROR_MESSAGE: &str = "wildcard (`*`) dependency constraints are not allowed \
     on crates.io. See https://doc.rust-lang.org/cargo/faq.html#can-\
     libraries-use--as-a-version-for-their-dependencies for more \
     information";

/// Handles the `PUT /crates/new` route.
/// Used by `cargo publish` to publish a new crate or to publish a new version of an
/// existing crate.
///
/// Currently blocks the HTTP thread, perhaps some function calls can spawn new
/// threads and return completion or error through other methods  a `cargo publish
/// --status` command, via crates.io's front end, or email.
pub fn publish(req: &mut dyn RequestExt) -> EndpointResult {
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

    req.log_metadata("crate_name", new_crate.name.to_string());
    req.log_metadata("crate_version", new_crate.vers.to_string());

    let conn = app.primary_database.get()?;
    let ids = req.authenticate()?;
    let api_token_id = ids.api_token_id();
    let user = ids.user();

    let verified_email_address = user.verified_email(&conn)?;
    let verified_email_address = verified_email_address.ok_or_else(|| {
        cargo_err(&format!(
            "A verified email address is required to publish crates to crates.io. \
             Visit https://{}/me to set and verify your email address.",
            app.config.domain_name,
        ))
    })?;

    // Create a transaction on the database, if there are no errors,
    // commit the transactions to record a new or updated crate.
    conn.transaction(|| {
        let _ = &new_crate;
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
            return Err(cargo_err(MISSING_RIGHTS_ERROR_MESSAGE));
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
            .ok_or_else(|| cargo_err("missing header: Content-Length"))?;

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
        .save(&conn, &verified_email_address)?;

        insert_version_owner_action(
            &conn,
            version.id,
            user.id,
            api_token_id,
            VersionAction::Publish,
        )?;

        // Link this new version to all dependencies
        let git_deps = add_dependencies(&conn, &new_crate.deps, version.id)?;

        // Update all keywords for this crate
        Keyword::update_crate(&conn, &krate, &keywords)?;

        // Update all categories for this crate, collecting any invalid categories
        // in order to be able to warn about them
        let ignored_invalid_categories = Category::update_crate(&conn, &krate, &categories)?;

        // Update all badges for this crate, collecting any invalid badges in
        // order to be able to warn about them
        let ignored_invalid_badges = Badge::update_crate(&conn, &krate, new_crate.badges.as_ref())?;
        let top_versions = krate.top_versions(&conn)?;

        // Read tarball from request
        let mut tarball = Vec::new();
        LimitErrorReader::new(req.body(), maximums.max_upload_size).read_to_end(&mut tarball)?;
        let hex_cksum: String = Sha256::digest(&tarball).encode_hex();
        let pkg_name = format!("{}-{}", krate.name, vers);
        let cargo_vcs_info = verify_tarball(&pkg_name, &tarball, maximums.max_unpack_size)?;
        let pkg_path_in_vcs = cargo_vcs_info.map(|info| info.path_in_vcs);

        if let Some(readme) = new_crate.readme {
            worker::render_and_upload_readme(
                version.id,
                readme,
                new_crate
                    .readme_file
                    .unwrap_or_else(|| String::from("README.md")),
                repo,
                pkg_path_in_vcs,
            )
            .enqueue(&conn)?;
        }

        // Upload crate tarball
        app.config
            .uploader()
            .upload_crate(&app, tarball, &krate, vers)?;

        let (features, features2): (HashMap<_, _>, HashMap<_, _>) =
            features.into_iter().partition(|(_k, vals)| {
                !vals
                    .iter()
                    .any(|v| v.starts_with("dep:") || v.contains("?/"))
            });
        let (features2, v) = if features2.is_empty() {
            (None, None)
        } else {
            (Some(features2), Some(2))
        };

        // Register this crate in our local git repo.
        let git_crate = git::Crate {
            name: name.0,
            vers: vers.to_string(),
            cksum: hex_cksum,
            features,
            features2,
            deps: git_deps,
            yanked: Some(false),
            links,
            v,
        };
        worker::add_crate(git_crate).enqueue(&conn)?;

        // The `other` field on `PublishWarnings` was introduced to handle a temporary warning
        // that is no longer needed. As such, crates.io currently does not return any `other`
        // warnings at this time, but if we need to, the field is available.
        let warnings = PublishWarnings {
            invalid_categories: ignored_invalid_categories,
            invalid_badges: ignored_invalid_badges,
            other: vec![],
        };

        Ok(req.json(&GoodCrate {
            krate: EncodableCrate::from_minimal(krate, &top_versions, None, false, None),
            warnings,
        }))
    })
}

/// Used by the `krate::new` function.
///
/// This function parses the JSON headers to interpret the data and validates
/// the data during and after the parsing. Returns crate metadata.
fn parse_new_headers(req: &mut dyn RequestExt) -> AppResult<EncodableCrateUpload> {
    // Read the json upload request
    let metadata_length = u64::from(read_le_u32(req.body())?);
    req.log_metadata("metadata_length", metadata_length);

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

    // It can have up to three elements per below conditions.
    let mut missing = Vec::with_capacity(3);

    if empty(new.description.as_ref()) {
        missing.push("description");
    }
    if empty(new.license.as_ref()) && empty(new.license_file.as_ref()) {
        missing.push("license");
    }
    if !missing.is_empty() {
        let message = missing_metadata_error_message(&missing);
        return Err(cargo_err(&message));
    }

    Ok(new)
}

pub fn missing_metadata_error_message(missing: &[&str]) -> String {
    format!(
        "missing or empty metadata fields: {}. Please \
         see https://doc.rust-lang.org/cargo/reference/manifest.html for \
         how to upload metadata",
        missing.join(", ")
    )
}

pub fn add_dependencies(
    conn: &PgConnection,
    deps: &[EncodableCrateDependency],
    target_version_id: i32,
) -> AppResult<Vec<git::Dependency>> {
    use self::dependencies::dsl::*;
    use diesel::insert_into;

    let git_and_new_dependencies = deps
        .iter()
        .map(|dep| {
            if let Some(registry) = &dep.registry {
                if !registry.is_empty() {
                    return Err(cargo_err(&format_args!("Dependency `{}` is hosted on another registry. Cross-registry dependencies are not permitted on crates.io.", &*dep.name)));
                }
            }

            // Match only identical names to ensure the index always references the original crate name
            let krate:Crate = Crate::by_exact_name(&dep.name)
                .first(&*conn)
                .map_err(|_| cargo_err(&format_args!("no known crate named `{}`", &*dep.name)))?;

            if let Ok(version_req) = semver::VersionReq::parse(&dep.version_req.0) {
                if version_req == semver::VersionReq::STAR {
                    return Err(cargo_err(WILDCARD_ERROR_MESSAGE));
                }
            }

            // If this dependency has an explicit name in `Cargo.toml` that
            // means that the `name` we have listed is actually the package name
            // that we're depending on. The `name` listed in the index is the
            // Cargo.toml-written-name which is what cargo uses for
            // `--extern foo=...`
            let (name, package) = match &dep.explicit_name_in_toml {
                Some(explicit) => (explicit.to_string(), Some(dep.name.to_string())),
                None => (dep.name.to_string(), None),
            };

            Ok((
                git::Dependency {
                    name,
                    req: dep.version_req.to_string(),
                    features: dep.features.iter().map(|s| s.0.to_string()).collect(),
                    optional: dep.optional,
                    default_features: dep.default_features,
                    target: dep.target.clone(),
                    kind: dep.kind.or(Some(DependencyKind::Normal)),
                    package,
                },
                (
                    version_id.eq(target_version_id),
                    crate_id.eq(krate.id),
                    req.eq(dep.version_req.to_string()),
                    dep.kind.map(|k| kind.eq(k as i32)),
                    optional.eq(dep.optional),
                    default_features.eq(dep.default_features),
                    features.eq(&dep.features),
                    target.eq(dep.target.as_deref()),
                ),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    let (git_deps, new_dependencies): (Vec<_>, Vec<_>) =
        git_and_new_dependencies.into_iter().unzip();

    insert_into(dependencies)
        .values(&new_dependencies)
        .execute(conn)?;

    Ok(git_deps)
}

fn verify_tarball(
    pkg_name: &str,
    tarball: &[u8],
    max_unpack: u64,
) -> AppResult<Option<CargoVcsInfo>> {
    // All our data is currently encoded with gzip
    let decoder = GzDecoder::new(tarball);

    // Don't let gzip decompression go into the weeeds, apply a fixed cap after
    // which point we say the decompressed source is "too large".
    let decoder = LimitErrorReader::new(decoder, max_unpack);

    // Use this I/O object now to take a peek inside
    let mut archive = tar::Archive::new(decoder);

    let vcs_info_path = Path::new(&pkg_name).join(".cargo_vcs_info.json");
    let mut vcs_info = None;

    for entry in archive.entries()? {
        let mut entry = entry.map_err(|err| {
            err.chain(cargo_err(
                "uploaded tarball is malformed or too large when decompressed",
            ))
        })?;

        // Verify that all entries actually start with `$name-$vers/`.
        // Historically Cargo didn't verify this on extraction so you could
        // upload a tarball that contains both `foo-0.1.0/` source code as well
        // as `bar-0.1.0/` source code, and this could overwrite other crates in
        // the registry!
        let entry_path = entry.path()?;
        if !entry_path.starts_with(&pkg_name) {
            return Err(cargo_err("invalid tarball uploaded"));
        }
        if entry_path == vcs_info_path {
            let mut contents = String::new();
            entry.read_to_string(&mut contents)?;
            vcs_info = CargoVcsInfo::from_contents(&contents).ok();
        }

        // Historical versions of the `tar` crate which Cargo uses internally
        // don't properly prevent hard links and symlinks from overwriting
        // arbitrary files on the filesystem. As a bit of a hammer we reject any
        // tarball with these sorts of links. Cargo doesn't currently ever
        // generate a tarball with these file types so this should work for now.
        let entry_type = entry.header().entry_type();
        if entry_type.is_hard_link() || entry_type.is_symlink() {
            return Err(cargo_err("invalid tarball uploaded"));
        }
    }
    Ok(vcs_info)
}

#[cfg(test)]
mod tests {
    use super::{missing_metadata_error_message, verify_tarball};
    use crate::admin::render_readmes::tests::add_file;
    use flate2::read::GzEncoder;
    use std::io::Read;

    #[test]
    fn missing_metadata_error_message_test() {
        assert_eq!(missing_metadata_error_message(&["a"]), "missing or empty metadata fields: a. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
        assert_eq!(missing_metadata_error_message(&["a", "b"]), "missing or empty metadata fields: a, b. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
        assert_eq!(missing_metadata_error_message(&["a", "b", "c"]), "missing or empty metadata fields: a, b, c. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
    }

    #[test]
    fn verify_tarball_test() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(&mut pkg, "foo-0.0.1/Cargo.toml", b"");
        let mut serialized_archive = vec![];
        GzEncoder::new(pkg.into_inner().unwrap().as_slice(), Default::default())
            .read_to_end(&mut serialized_archive)
            .unwrap();

        let limit = 512 * 1024 * 1024;
        assert_eq!(
            verify_tarball("foo-0.0.1", &serialized_archive, limit).unwrap(),
            None
        );
        assert_err!(verify_tarball("bar-0.0.1", &serialized_archive, limit));
    }

    #[test]
    fn verify_tarball_test_incomplete_vcs_info() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(&mut pkg, "foo-0.0.1/Cargo.toml", b"");
        add_file(
            &mut pkg,
            "foo-0.0.1/.cargo_vcs_info.json",
            br#"{"unknown": "field"}"#,
        );
        let mut serialized_archive = vec![];
        GzEncoder::new(pkg.into_inner().unwrap().as_slice(), Default::default())
            .read_to_end(&mut serialized_archive)
            .unwrap();
        let limit = 512 * 1024 * 1024;
        let vcs_info = verify_tarball("foo-0.0.1", &serialized_archive, limit)
            .unwrap()
            .unwrap();
        assert_eq!(vcs_info.path_in_vcs, "");
    }

    #[test]
    fn verify_tarball_test_vcs_info() {
        let mut pkg = tar::Builder::new(vec![]);
        add_file(&mut pkg, "foo-0.0.1/Cargo.toml", b"");
        add_file(
            &mut pkg,
            "foo-0.0.1/.cargo_vcs_info.json",
            br#"{"path_in_vcs": "path/in/vcs"}"#,
        );
        let mut serialized_archive = vec![];
        GzEncoder::new(pkg.into_inner().unwrap().as_slice(), Default::default())
            .read_to_end(&mut serialized_archive)
            .unwrap();
        let limit = 512 * 1024 * 1024;
        let vcs_info = verify_tarball("foo-0.0.1", &serialized_archive, limit)
            .unwrap()
            .unwrap();
        assert_eq!(vcs_info.path_in_vcs, "path/in/vcs");
    }
}
