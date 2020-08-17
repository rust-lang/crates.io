//! Functionality related to publishing a new crate or version of a crate.

use hex::ToHex;
use std::sync::Arc;
use swirl::Job;

use crate::controllers::cargo_prelude::*;
use crate::git;
use crate::models::{
    insert_version_owner_action, Badge, Category, Crate, DependencyKind, Keyword, NewCrate,
    NewVersion, Rights, VersionAction,
};

use crate::render;
use crate::schema::*;
use crate::util::errors::{cargo_err, AppResult};
use crate::util::{read_fill, read_le_u32, Maximums};
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

        if let Some(readme) = new_crate.readme {
            render::render_and_upload_readme(
                version.id,
                readme,
                new_crate
                    .readme_file
                    .unwrap_or_else(|| String::from("README.md")),
                repo,
            )
            .enqueue(&conn)?;
        }

        let cksum = app
            .config
            .uploader
            .upload_crate(req, &krate, maximums, vers)?;

        let hex_cksum = cksum.encode_hex::<String>();

        // Register this crate in our local git repo and send notification emails
        // to owners who haven't opted out of them.
        let git_crate = git::Crate {
            name: name.0,
            vers: vers.to_string(),
            cksum: hex_cksum,
            features,
            deps: git_deps,
            yanked: Some(false),
            links,
        };
        let emails = krate.owners_with_notification_email(&conn)?;
        git::add_crate(git_crate, emails, user.name, verified_email_address).enqueue(&conn)?;

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
            if semver::VersionReq::parse(&dep.version_req.0) == semver::VersionReq::parse("*") {
                return Err(cargo_err(WILDCARD_ERROR_MESSAGE));
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

#[cfg(test)]
mod tests {
    use super::missing_metadata_error_message;

    #[test]
    fn missing_metadata_error_message_test() {
        assert_eq!(missing_metadata_error_message(&["a"]), "missing or empty metadata fields: a. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
        assert_eq!(missing_metadata_error_message(&["a", "b"]), "missing or empty metadata fields: a, b. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
        assert_eq!(missing_metadata_error_message(&["a", "b", "c"]), "missing or empty metadata fields: a, b, c. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for how to upload metadata");
    }
}
