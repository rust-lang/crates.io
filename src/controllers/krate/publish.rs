//! Functionality related to publishing a new crate or version of a crate.

use crate::auth::AuthCheck;
use axum::body::Bytes;
use flate2::read::GzDecoder;
use hex::ToHex;
use hyper::body::Buf;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use crate::controllers::cargo_prelude::*;
use crate::controllers::util::RequestPartsExt;
use crate::models::{
    insert_version_owner_action, Category, Crate, DependencyKind, Keyword, NewCrate, NewVersion,
    Rights, VersionAction,
};
use crate::worker;

use crate::middleware::log_request::RequestLogExt;
use crate::models::token::EndpointScope;
use crate::schema::*;
use crate::util::errors::{cargo_err, AppResult};
use crate::util::{CargoVcsInfo, LimitErrorReader, Maximums};
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
pub async fn publish(app: AppState, req: BytesRequest) -> AppResult<Json<GoodCrate>> {
    let (req, bytes) = req.0.into_parts();
    let (json_bytes, tarball_bytes) = split_body(bytes, &req)?;

    let new_crate: EncodableCrateUpload = serde_json::from_slice(&json_bytes)
        .map_err(|e| cargo_err(&format_args!("invalid upload request: {e}")))?;

    let request_log = req.request_log();
    request_log.add("crate_name", new_crate.name.to_string());
    request_log.add("crate_version", new_crate.vers.to_string());

    // Make sure required fields are provided
    fn empty(s: Option<&String>) -> bool {
        s.map_or(true, String::is_empty)
    }

    // It can have up to three elements per below conditions.
    let mut missing = Vec::with_capacity(3);

    if empty(new_crate.description.as_ref()) {
        missing.push("description");
    }
    if empty(new_crate.license.as_ref()) && empty(new_crate.license_file.as_ref()) {
        missing.push("license");
    }
    if !missing.is_empty() {
        let message = missing_metadata_error_message(&missing);
        return Err(cargo_err(&message));
    }

    conduit_compat(move || {
        let conn = &mut *app.primary_database.get()?;

        // this query should only be used for the endpoint scope calculation
        // since a race condition there would only cause `publish-new` instead of
        // `publish-update` to be used.
        let existing_crate = Crate::by_name(&new_crate.name)
            .first::<Crate>(conn)
            .optional()?;

        let endpoint_scope = match existing_crate {
            Some(_) => EndpointScope::PublishUpdate,
            None => EndpointScope::PublishNew,
        };

        let auth = AuthCheck::default()
            .with_endpoint_scope(endpoint_scope)
            .for_crate(&new_crate.name)
            .check(&req, conn)?;

        let api_token_id = auth.api_token_id();
        let user = auth.user();

        let verified_email_address = user.verified_email(conn)?;
        let verified_email_address = verified_email_address.ok_or_else(|| {
            cargo_err(&format!(
                "A verified email address is required to publish crates to crates.io. \
             Visit https://{}/me to set and verify your email address.",
                app.config.domain_name,
            ))
        })?;

        // Create a transaction on the database, if there are no errors,
        // commit the transactions to record a new or updated crate.
        conn.transaction(|conn| {
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
                persist.create_or_update(conn, user.id, Some(&app.config.publish_rate_limit))?;

            let owners = krate.owners(conn)?;
            if user.rights(&app, &owners)? < Rights::Publish {
                return Err(cargo_err(MISSING_RIGHTS_ERROR_MESSAGE));
            }

            if krate.name != *name {
                return Err(cargo_err(&format_args!(
                    "crate was previously named `{}`",
                    krate.name
                )));
            }

            if let Some(daily_version_limit) = app.config.new_version_rate_limit {
                let published_today = count_versions_published_today(krate.id, conn)?;
                if published_today >= daily_version_limit as i64 {
                    return Err(cargo_err(
                        "You have published too many versions of this crate in the last 24 hours",
                    ));
                }
            }

            let content_length = tarball_bytes.len() as u64;

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

            // Read tarball from request
            let hex_cksum: String = Sha256::digest(&tarball_bytes).encode_hex();

            // Persist the new version of this crate
            let version = NewVersion::new(
                krate.id,
                vers,
                &features,
                license,
                license_file,
                // Downcast is okay because the file length must be less than the max upload size
                // to get here, and max upload sizes are way less than i32 max
                content_length as i32,
                user.id,
                hex_cksum.clone(),
                links.clone(),
            )?
            .save(conn, &verified_email_address)?;

            insert_version_owner_action(
                conn,
                version.id,
                user.id,
                api_token_id,
                VersionAction::Publish,
            )?;

            // Link this new version to all dependencies
            let git_deps = add_dependencies(conn, &new_crate.deps, version.id)?;

            // Update all keywords for this crate
            Keyword::update_crate(conn, &krate, &keywords)?;

            // Update all categories for this crate, collecting any invalid categories
            // in order to be able to warn about them
            let ignored_invalid_categories = Category::update_crate(conn, &krate, &categories)?;

            let top_versions = krate.top_versions(conn)?;

            let pkg_name = format!("{}-{}", krate.name, vers);
            let cargo_vcs_info =
                verify_tarball(&pkg_name, &tarball_bytes, maximums.max_unpack_size)?;
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
                .enqueue(conn)?;
            }

            // Upload crate tarball
            app.config
                .uploader()
                .upload_crate(app.http_client(), tarball_bytes, &krate, vers)?;

            let uses_features2_syntax = features
                .iter()
                .flat_map(|(_key, values)| values)
                .any(|values| values.starts_with("dep:") || values.contains("?/"));

            let (features, features2) = match uses_features2_syntax {
                true => (BTreeMap::new(), Some(features)),
                false => (features, None),
            };

            let v = features2.as_ref().map(|_| 2);

            // Register this crate in our local git repo.
            let git_crate = cargo_registry_index::Crate {
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
            worker::add_crate(git_crate).enqueue(conn)?;

            // The `other` field on `PublishWarnings` was introduced to handle a temporary warning
            // that is no longer needed. As such, crates.io currently does not return any `other`
            // warnings at this time, but if we need to, the field is available.
            let warnings = PublishWarnings {
                invalid_categories: ignored_invalid_categories,
                invalid_badges: vec![],
                other: vec![],
            };

            Ok(Json(GoodCrate {
                krate: EncodableCrate::from_minimal(krate, Some(&top_versions), None, false, None),
                warnings,
            }))
        })
    })
    .await
}

/// Counts the number of versions for `krate_id` that were published within
/// the last 24 hours.
fn count_versions_published_today(krate_id: i32, conn: &mut PgConnection) -> QueryResult<i64> {
    use crate::schema::versions::dsl::*;
    use diesel::dsl::{now, IntervalDsl};

    versions
        .filter(crate_id.eq(krate_id))
        .filter(created_at.gt(now - 24.hours()))
        .count()
        .get_result(conn)
}

#[instrument(skip_all)]
fn split_body<R: RequestPartsExt>(mut bytes: Bytes, req: &R) -> AppResult<(Bytes, Bytes)> {
    // The format of the req.body() of a publish request is as follows:
    //
    // metadata length
    // metadata in JSON about the crate being published
    // .crate tarball length
    // .crate tarball file

    let json_len = bytes.get_u32_le() as usize;
    req.request_log().add("metadata_length", json_len);

    if json_len > bytes.len() {
        return Err(cargo_err(&format!(
            "invalid metadata length for remaining payload: {json_len}"
        )));
    }

    let json_bytes = bytes.split_to(json_len);

    let tarball_len = bytes.get_u32_le() as usize;
    if tarball_len > bytes.len() {
        return Err(cargo_err(&format!(
            "invalid metadata length for remaining payload: {tarball_len}"
        )));
    }

    let tarball_bytes = bytes.split_to(tarball_len);

    Ok((json_bytes, tarball_bytes))
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
    conn: &mut PgConnection,
    deps: &[EncodableCrateDependency],
    target_version_id: i32,
) -> AppResult<Vec<cargo_registry_index::Dependency>> {
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
                .first(conn)
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
                cargo_registry_index::Dependency {
                    name,
                    req: dep.version_req.to_string(),
                    features: dep.features.iter().map(|s| s.0.to_string()).collect(),
                    optional: dep.optional,
                    default_features: dep.default_features,
                    target: dep.target.clone(),
                    kind: dep.kind.or(Some(DependencyKind::Normal)).map(|dk| dk.into()),
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
                    explicit_name.eq(dep.explicit_name_in_toml.as_deref())
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
        if !entry_path.starts_with(pkg_name) {
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
