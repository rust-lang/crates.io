//! Functionality related to publishing a new crate or version of a crate.

use crate::app::AppState;
use crate::auth::AuthCheck;
use crate::util::diesel::prelude::*;
use crate::worker::jobs::{
    self, CheckTyposquat, SendPublishNotificationsJob, UpdateDefaultVersion,
};
use axum::body::Bytes;
use axum::Json;
use cargo_manifest::{Dependency, DepsSet, TargetDepsSet};
use chrono::{DateTime, SecondsFormat, Utc};
use crates_io_tarball::{process_tarball, TarballError};
use crates_io_worker::BackgroundJob;
use diesel::connection::DefaultLoadingMode;
use diesel::dsl::{exists, select};
use diesel_async::async_connection_wrapper::AsyncConnectionWrapper;
use hex::ToHex;
use http::StatusCode;
use hyper::body::Buf;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use tokio::runtime::Handle;
use url::Url;

use crate::models::{
    default_versions::Version as DefaultVersion, Category, Crate, DependencyKind, Keyword,
    NewCrate, NewVersion, NewVersionOwnerAction, Rights, VersionAction,
};

use crate::licenses::parse_license_expr;
use crate::middleware::log_request::RequestLogExt;
use crate::models::token::EndpointScope;
use crate::rate_limiter::LimitedAction;
use crate::schema::*;
use crate::sql::canon_crate_name;
use crate::tasks::spawn_blocking;
use crate::util::diesel::Conn;
use crate::util::errors::{bad_request, custom, internal, AppResult, BoxedAppError};
use crate::util::{BytesRequest, Maximums};
use crate::views::{
    EncodableCrate, EncodableCrateDependency, GoodCrate, PublishMetadata, PublishWarnings,
};

const MISSING_RIGHTS_ERROR_MESSAGE: &str = "this crate exists but you don't seem to be an owner. \
     If you believe this is a mistake, perhaps you need \
     to accept an invitation to be an owner before \
     publishing.";

const MAX_DESCRIPTION_LENGTH: usize = 1000;

/// Handles the `PUT /crates/new` route.
/// Used by `cargo publish` to publish a new crate or to publish a new version of an
/// existing crate.
///
/// Currently blocks the HTTP thread, perhaps some function calls can spawn new
/// threads and return completion or error through other methods  a `cargo publish
/// --status` command, via crates.io's front end, or email.
pub async fn publish(app: AppState, req: BytesRequest) -> AppResult<Json<GoodCrate>> {
    let (req, bytes) = req.0.into_parts();
    let (json_bytes, tarball_bytes) = split_body(bytes)?;

    let metadata: PublishMetadata = serde_json::from_slice(&json_bytes)
        .map_err(|e| bad_request(format_args!("invalid upload request: {e}")))?;

    Crate::validate_crate_name("crate", &metadata.name).map_err(bad_request)?;

    let semver = match semver::Version::parse(&metadata.vers) {
        Ok(parsed) => parsed,
        Err(_) => {
            return Err(bad_request(format_args!(
                "\"{}\" is an invalid semver version",
                metadata.vers
            )))
        }
    };

    // Convert the version back to a string to deal with any inconsistencies
    let version_string = semver.to_string();

    let request_log = req.request_log();
    request_log.add("crate_name", &*metadata.name);
    request_log.add("crate_version", &version_string);

    let mut conn = app.db_write().await?;

    let (existing_crate, auth) = {
        use diesel_async::RunQueryDsl;

        let deleted_crate: Option<(String, DateTime<Utc>)> = deleted_crates::table
            .filter(canon_crate_name(deleted_crates::name).eq(canon_crate_name(&metadata.name)))
            .filter(deleted_crates::available_at.gt(Utc::now()))
            .select((deleted_crates::name, deleted_crates::available_at))
            .first(&mut conn)
            .await
            .optional()?;

        if let Some(deleted_crate) = deleted_crate {
            return Err(bad_request(format!(
                "A crate with the name `{}` was recently deleted. Reuse of this name will be available after {}.",
                deleted_crate.0,
                deleted_crate.1.to_rfc3339_opts(SecondsFormat::Secs, true)
            )));
        }

        // this query should only be used for the endpoint scope calculation
        // since a race condition there would only cause `publish-new` instead of
        // `publish-update` to be used.
        let existing_crate: Option<Crate> = Crate::by_name(&metadata.name)
            .first::<Crate>(&mut conn)
            .await
            .optional()?;

        let endpoint_scope = match existing_crate {
            Some(_) => EndpointScope::PublishUpdate,
            None => EndpointScope::PublishNew,
        };

        let auth = AuthCheck::default()
            .with_endpoint_scope(endpoint_scope)
            .for_crate(&metadata.name)
            .check(&req, &mut conn)
            .await?;
        (existing_crate, auth)
    };

    let verified_email_address = auth.user().async_verified_email(&mut conn).await?;
    let verified_email_address = verified_email_address.ok_or_else(|| {
        bad_request(format!(
            "A verified email address is required to publish crates to crates.io. \
             Visit https://{}/settings/profile to set and verify your email address.",
            app.config.domain_name,
        ))
    })?;

    // Use a different rate limit whether this is a new or an existing crate.
    let rate_limit_action = match existing_crate {
        Some(_) => LimitedAction::PublishUpdate,
        None => LimitedAction::PublishNew,
    };

    app.rate_limiter
        .check_rate_limit(auth.user().id, rate_limit_action, &mut conn)
        .await?;

    spawn_blocking(move || {
        use diesel::RunQueryDsl;

        let conn: &mut AsyncConnectionWrapper<_> = &mut conn.into();

        let api_token_id = auth.api_token_id();
        let user = auth.user();

        let content_length = tarball_bytes.len() as u64;

        let maximums = Maximums::new(
            existing_crate.as_ref().and_then(|c| c.max_upload_size),
            app.config.max_upload_size,
            app.config.max_unpack_size,
        );

        if content_length > maximums.max_upload_size {
            return Err(custom(StatusCode::PAYLOAD_TOO_LARGE, format!(
                "max upload size is: {}",
                maximums.max_upload_size
            )));
        }

        let pkg_name = format!("{}-{}", &*metadata.name, &version_string);
        let tarball_info = process_tarball(&pkg_name, &*tarball_bytes, maximums.max_unpack_size)?;

        // `unwrap()` is safe here since `process_tarball()` validates that
        // we only accept manifests with a `package` section and without
        // inheritance.
        let package = tarball_info.manifest.package.unwrap();

        let description = package.description.map(|it| it.as_local().unwrap());
        let mut license = package.license.map(|it| it.as_local().unwrap());
        let license_file = package.license_file.map(|it| it.as_local().unwrap());
        let homepage = package.homepage.map(|it| it.as_local().unwrap());
        let documentation = package.documentation.map(|it| it.as_local().unwrap());
        let repository = package.repository.map(|it| it.as_local().unwrap());
        let rust_version = package.rust_version.map(|rv| rv.as_local().unwrap());
        let edition = package.edition.map(|rv| rv.as_local().unwrap());

        // Make sure required fields are provided
        fn empty(s: Option<&String>) -> bool {
            s.map_or(true, String::is_empty)
        }

        // It can have up to three elements per below conditions.
        let mut missing = Vec::with_capacity(3);
        if empty(description.as_ref()) {
            missing.push("description");
        }
        if empty(license.as_ref()) && empty(license_file.as_ref()) {
            missing.push("license");
        }
        if !missing.is_empty() {
            let message = missing_metadata_error_message(&missing);
            return Err(bad_request(&message));
        }

        if let Some(description) = &description {
            if description.len() > MAX_DESCRIPTION_LENGTH {
                return Err(bad_request(format!("The `description` is too long. A maximum of {MAX_DESCRIPTION_LENGTH} characters are currently allowed.")));
            }
        }

        if let Some(ref license) = license {
            parse_license_expr(license).map_err(|e| bad_request(format_args!(
                "unknown or invalid license expression; \
                see http://opensource.org/licenses for options, \
                and http://spdx.org/licenses/ for their identifiers\n\
                Note: If you have a non-standard license that is not listed by SPDX, \
                use the license-file field to specify the path to a file containing \
                the text of the license.\n\
                See https://doc.rust-lang.org/cargo/reference/manifest.html#the-license-and-license-file-fields \
                for more information.\n\
                {e}"
            )))?;
        } else if license_file.is_some() {
            // If no license is given, but a license file is given, flag this
            // crate as having a nonstandard license. Note that we don't
            // actually do anything else with license_file currently.
            license = Some(String::from("non-standard"));
        }

        validate_url(homepage.as_deref(), "homepage")?;
        validate_url(documentation.as_deref(), "documentation")?;
        validate_url(repository.as_deref(), "repository")?;
        if let Some(ref rust_version) = rust_version {
            validate_rust_version(rust_version)?;
        }

        let keywords = package
            .keywords
            .map(|it| it.as_local().unwrap())
            .unwrap_or_default();

        if keywords.len() > 5 {
            return Err(bad_request("expected at most 5 keywords per crate"));
        }

        for keyword in keywords.iter() {
            if keyword.len() > 20 {
                return Err(bad_request(format!(
                    "\"{keyword}\" is an invalid keyword (keywords must have less than 20 characters)"
                )));
            } else if !Keyword::valid_name(keyword) {
                return Err(bad_request(format!("\"{keyword}\" is an invalid keyword")));
            }
        }

        let categories = package
            .categories
            .map(|it| it.as_local().unwrap())
            .unwrap_or_default();

        if categories.len() > 5 {
            return Err(bad_request("expected at most 5 categories per crate"));
        }

        let max_features = existing_crate.as_ref()
            .and_then(|c| c.max_features.map(|mf| mf as usize))
            .unwrap_or(app.config.max_features);

        let features = tarball_info.manifest.features.unwrap_or_default();
        let num_features = features.len();
        if num_features > max_features {
            return Err(bad_request(format!(
                "crates.io only allows a maximum number of {max_features} \
                features, but your crate is declaring {num_features} features.\n\
                \n\
                Take a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html \
                to understand why this restriction was introduced.\n\
                \n\
                If you have a use case that requires an increase of this limit, \
                please send us an email to help@crates.io to discuss the details."
            )));
        }

        for (key, values) in features.iter() {
            Crate::validate_feature_name(key).map_err(bad_request)?;

            let num_features = values.len();
            if num_features > max_features {
                return Err(bad_request(format!(
                    "crates.io only allows a maximum number of {max_features} \
                    features or dependencies that another feature can enable, \
                    but the \"{key}\" feature of your crate is enabling \
                    {num_features} features or dependencies.\n\
                    \n\
                    Take a look at https://blog.rust-lang.org/2023/10/26/broken-badges-and-23k-keywords.html \
                    to understand why this restriction was introduced.\n\
                    \n\
                    If you have a use case that requires an increase of this limit, \
                    please send us an email to help@crates.io to discuss the details."
                )));
            }

            for value in values.iter() {
                Crate::validate_feature(value).map_err(bad_request)?;
            }
        }

        let deps = convert_dependencies(
            tarball_info.manifest.dependencies.as_ref(),
            tarball_info.manifest.dev_dependencies.as_ref(),
            tarball_info.manifest.build_dependencies.as_ref(),
            tarball_info.manifest.target.as_ref()
        );

        let max_dependencies = app.config.max_dependencies;
        if deps.len() > max_dependencies {
            return Err(bad_request(format!(
                "crates.io only allows a maximum number of {max_dependencies} dependencies.\n\
                \n\
                If you have a use case that requires an increase of this limit, \
                please send us an email to help@crates.io to discuss the details."
            )));
        }

        for dep in &deps {
            validate_dependency(dep)?;
        }

        // Create a transaction on the database, if there are no errors,
        // commit the transactions to record a new or updated crate.
        conn.transaction(|conn| {
            let name = metadata.name;
            let keywords = keywords.iter().map(|s| s.as_str()).collect::<Vec<_>>();
            let categories = categories.iter().map(|s| s.as_str()).collect::<Vec<_>>();

            // Persist the new crate, if it doesn't already exist
            let persist = NewCrate {
                name: &name,
                description: description.as_deref(),
                homepage: homepage.as_deref(),
                documentation: documentation.as_deref(),
                readme: metadata.readme.as_deref(),
                repository: repository.as_deref(),
                max_upload_size: None,
                max_features: None,
            };

            if is_reserved_name(persist.name, conn)? {
                return Err(bad_request("cannot upload a crate with a reserved name"));
            }

            // To avoid race conditions, we try to insert
            // first so we know whether to add an owner
            let krate = match persist.create(conn, user.id).optional()? {
                Some(krate) => krate,
                None => persist.update(conn)?,
            };

            let owners = krate.owners(conn)?;
            if Handle::current().block_on(user.rights(&app, &owners))? < Rights::Publish {
                return Err(custom(StatusCode::FORBIDDEN, MISSING_RIGHTS_ERROR_MESSAGE));
            }

            if krate.name != *name {
                return Err(bad_request(format_args!(
                    "crate was previously named `{}`",
                    krate.name
                )));
            }

            if let Some(daily_version_limit) = app.config.new_version_rate_limit {
                let published_today = count_versions_published_today(krate.id, conn)?;
                if published_today >= daily_version_limit as i64 {
                    return Err(custom(
                        StatusCode::TOO_MANY_REQUESTS,
                        "You have published too many versions of this crate in the last 24 hours",
                    ));
                }
            }

            // https://doc.rust-lang.org/cargo/reference/cargo-targets.html#the-name-field says that
            // the `name` field is required for `bin` targets, so we can ignore `None` values via
            // `filter_map()` here.
            let bin_names = tarball_info.manifest.bin
                .iter()
                .filter_map(|bin| bin.name.as_deref())
                .collect::<Vec<_>>();

            let edition = edition.map(|edition| edition.as_str());

            // Read tarball from request
            let hex_cksum: String = Sha256::digest(&tarball_bytes).encode_hex();

            // Persist the new version of this crate
            let new_version = NewVersion::builder(krate.id, &version_string)
                .features(serde_json::to_value(&features)?)
                .maybe_license(license.as_deref())
                // Downcast is okay because the file length must be less than the max upload size
                // to get here, and max upload sizes are way less than i32 max
                .size(content_length as i32)
                .published_by(user.id)
                .checksum(&hex_cksum)
                .maybe_links(package.links.as_deref())
                .maybe_rust_version(rust_version.as_deref())
                .has_lib(tarball_info.manifest.lib.is_some())
                .bin_names(bin_names.as_slice())
                .maybe_edition(edition)
                .build();

            let version = new_version.save(conn, &verified_email_address).map_err(|error| {
                use diesel::result::{Error, DatabaseErrorKind};
                match error {
                    Error::DatabaseError(DatabaseErrorKind::UniqueViolation, _) =>
                        duplicate_version_error(new_version.num_no_build),
                    error => error.into(),
                }
            })?;

            NewVersionOwnerAction::builder()
                .version_id(version.id)
                .user_id(user.id)
                .maybe_api_token_id(api_token_id)
                .action(VersionAction::Publish)
                .build()
                .insert(conn)?;

            // Link this new version to all dependencies
            add_dependencies(conn, &deps, version.id)?;

            let existing_default_version = default_versions::table
                .inner_join(versions::table)
                .filter(default_versions::crate_id.eq(krate.id))
                .select(DefaultVersion::as_select())
                .first(conn)
                .optional()?;

            let mut default_version = None;
            // Upsert the `default_value` determined by the existing `default_value` and the
            // published version. Note that this could potentially write an outdated version
            // (although this should not happen regularly), as we might be comparing to an
            // outdated value.
            //
            // Compared to only using a background job, this prevents us from getting into a
            // situation where a crate exists in the `crates` table but doesn't have a default
            // version in the `default_versions` table.
            if let Some(existing_default_version) = existing_default_version {
                let published_default_version = DefaultVersion {
                    id: version.id,
                    num: semver,
                    yanked: false,
                };

                if existing_default_version < published_default_version {
                    diesel::update(default_versions::table)
                        .filter(default_versions::crate_id.eq(krate.id))
                        .set(default_versions::version_id.eq(version.id))
                        .execute(conn)?;
                } else {
                    default_version = Some(existing_default_version.num.to_string());
                }

                // Update the default version asynchronously in a background job
                // to ensure correctness and eventual consistency.
                UpdateDefaultVersion::new(krate.id).enqueue(conn)?;
            } else {
                diesel::insert_into(default_versions::table)
                    .values((
                        default_versions::crate_id.eq(krate.id),
                        default_versions::version_id.eq(version.id),
                    ))
                    .execute(conn)?;
            }

            // Update all keywords for this crate
            Keyword::update_crate(conn, krate.id, &keywords)?;

            // Update all categories for this crate, collecting any invalid categories
            // in order to be able to return an error to the user.
            let unknown_categories = Category::update_crate(conn, krate.id, &categories)?;
            if !unknown_categories.is_empty() {
                let unknown_categories = unknown_categories.join(", ");
                let domain = &app.config.domain_name;
                return Err(bad_request(format!("The following category slugs are not currently supported on crates.io: {}\n\nSee https://{}/category_slugs for a list of supported slugs.", unknown_categories, domain)));
            }

            let top_versions = krate.top_versions(conn)?;

            let downloads: i64 = crate_downloads::table.select(crate_downloads::downloads)
                .filter(crate_downloads::crate_id.eq(krate.id))
                .first(conn)?;

            let pkg_path_in_vcs = tarball_info.vcs_info.map(|info| info.path_in_vcs);

            if let Some(readme) = metadata.readme {
                if !readme.is_empty() {
                    jobs::RenderAndUploadReadme::new(
                        version.id,
                        readme,
                        metadata
                            .readme_file
                            .unwrap_or_else(|| String::from("README.md")),
                        repository,
                        pkg_path_in_vcs,
                    )
                        .enqueue(conn)?;
                }
            }

            // Upload crate tarball
            Handle::current()
                .block_on(app.storage.upload_crate_file(
                    &krate.name,
                    &version_string,
                    tarball_bytes,
                ))
                .map_err(|e| internal(format!("failed to upload crate: {e}")))?;

            jobs::SyncToGitIndex::new(&krate.name).enqueue(conn)?;
            jobs::SyncToSparseIndex::new(&krate.name).enqueue(conn)?;

            SendPublishNotificationsJob::new(version.id).enqueue(conn)?;

            // Experiment: check new crates for potential typosquatting.
            if existing_crate.is_none() {
                CheckTyposquat::new(&krate.name).enqueue(conn)?;
            }

            let job = jobs::rss::SyncCrateFeed::new(krate.name.clone());
            if let Err(error) = job.enqueue(conn) {
                error!("Failed to enqueue `rss::SyncCrateFeed` job: {error}");
            }

            if let Err(error) = jobs::rss::SyncUpdatesFeed.enqueue(conn) {
                error!("Failed to enqueue `rss::SyncUpdatesFeed` job: {error}");
            }

            if existing_crate.is_none() {
                if let Err(error) = jobs::rss::SyncCratesFeed.enqueue(conn) {
                    error!("Failed to enqueue `rss::SyncCratesFeed` job: {error}");
                }
            }

            // The `other` field on `PublishWarnings` was introduced to handle a temporary warning
            // that is no longer needed. As such, crates.io currently does not return any `other`
            // warnings at this time, but if we need to, the field is available.
            let warnings = PublishWarnings {
                invalid_categories: vec![],
                invalid_badges: vec![],
                other: vec![],
            };

            Ok(Json(GoodCrate {
                krate: EncodableCrate::from_minimal(
                    krate,
                    default_version.or(Some(version_string)).as_deref(),
                    Some(false),
                    Some(&top_versions),
                    false,
                    downloads,
                    None,
                ),
                warnings,
            }))
        })
    })
        .await
}

/// Counts the number of versions for `crate_id` that were published within
/// the last 24 hours.
fn count_versions_published_today(crate_id: i32, conn: &mut impl Conn) -> QueryResult<i64> {
    use diesel::dsl::{now, IntervalDsl};
    use diesel::RunQueryDsl;

    versions::table
        .filter(versions::crate_id.eq(crate_id))
        .filter(versions::created_at.gt(now - 24.hours()))
        .count()
        .get_result(conn)
}

#[instrument(skip_all)]
fn split_body(mut bytes: Bytes) -> AppResult<(Bytes, Bytes)> {
    // The format of the req.body() of a publish request is as follows:
    //
    // metadata length
    // metadata in JSON about the crate being published
    // .crate tarball length
    // .crate tarball file

    if bytes.len() < 4 {
        // Avoid panic in `get_u32_le()` if there is not enough remaining data
        return Err(bad_request("invalid metadata length"));
    }

    let json_len = bytes.get_u32_le() as usize;
    if json_len > bytes.len() {
        return Err(bad_request(format!(
            "invalid metadata length for remaining payload: {json_len}"
        )));
    }

    let json_bytes = bytes.split_to(json_len);

    if bytes.len() < 4 {
        // Avoid panic in `get_u32_le()` if there is not enough remaining data
        return Err(bad_request("invalid tarball length"));
    }

    let tarball_len = bytes.get_u32_le() as usize;
    if tarball_len > bytes.len() {
        return Err(bad_request(format!(
            "invalid tarball length for remaining payload: {tarball_len}"
        )));
    }

    let tarball_bytes = bytes.split_to(tarball_len);

    Ok((json_bytes, tarball_bytes))
}

fn is_reserved_name(name: &str, conn: &mut impl Conn) -> QueryResult<bool> {
    use diesel::RunQueryDsl;

    select(exists(reserved_crate_names::table.filter(
        canon_crate_name(reserved_crate_names::name).eq(canon_crate_name(name)),
    )))
    .get_result(conn)
}

fn validate_url(url: Option<&str>, field: &str) -> AppResult<()> {
    let Some(url) = url else {
        return Ok(());
    };

    // Manually check the string, as `Url::parse` may normalize relative URLs
    // making it difficult to ensure that both slashes are present.
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(bad_request(format_args!(
            "URL for field `{field}` must begin with http:// or https:// (url: {url})"
        )));
    }

    // Ensure the entire URL parses as well
    Url::parse(url)
        .map_err(|_| bad_request(format_args!("`{field}` is not a valid url: `{url}`")))?;
    Ok(())
}

fn missing_metadata_error_message(missing: &[&str]) -> String {
    format!(
        "missing or empty metadata fields: {}. Please \
         see https://doc.rust-lang.org/cargo/reference/manifest.html for \
         more information on configuring these fields",
        missing.join(", ")
    )
}

fn duplicate_version_error(version: &str) -> BoxedAppError {
    bad_request(format!("crate version `{version}` is already uploaded"))
}

fn validate_rust_version(value: &str) -> AppResult<()> {
    match semver::VersionReq::parse(value) {
        // Exclude semver operators like `^` and pre-release identifiers
        Ok(_) if value.chars().all(|c| c.is_ascii_digit() || c == '.') => Ok(()),
        Ok(_) | Err(..) => Err(bad_request(
            "failed to parse `Cargo.toml` manifest file\n\ninvalid `rust-version` value",
        )),
    }
}

fn convert_dependencies(
    normal_deps: Option<&DepsSet>,
    dev_deps: Option<&DepsSet>,
    build_deps: Option<&DepsSet>,
    targets: Option<&TargetDepsSet>,
) -> Vec<EncodableCrateDependency> {
    use DependencyKind as Kind;

    let mut result = vec![];

    let mut add = |deps_set: &DepsSet, kind: Kind, target: Option<&str>| {
        for (name, dep) in deps_set {
            result.push(convert_dependency(name, dep, kind, target));
        }
    };

    if let Some(deps) = normal_deps {
        add(deps, Kind::Normal, None);
    }
    if let Some(deps) = dev_deps {
        add(deps, Kind::Dev, None);
    }
    if let Some(deps_set) = build_deps {
        add(deps_set, Kind::Build, None);
    }
    if let Some(target_deps_set) = targets {
        for (target, deps) in target_deps_set {
            add(&deps.dependencies, Kind::Normal, Some(target));
            add(&deps.dev_dependencies, Kind::Dev, Some(target));
            add(&deps.build_dependencies, Kind::Build, Some(target));
        }
    }

    result
}

fn convert_dependency(
    name: &str,
    dep: &Dependency,
    kind: DependencyKind,
    target: Option<&str>,
) -> EncodableCrateDependency {
    let details = dep.detail();

    // Normalize version requirement with a `parse()` and `to_string()` cycle.
    //
    // If the value can't be parsed the `validate_dependency()` fn will return
    // an error later in the call chain. Parsing the value twice is a bit
    // wasteful, but we can clean this up later.
    let req = semver::VersionReq::parse(dep.req())
        .map(|req| req.to_string())
        .unwrap_or_else(|_| dep.req().to_string());

    let (crate_name, explicit_name_in_toml) = match details.and_then(|it| it.package.clone()) {
        None => (name.to_string(), None),
        Some(package) => (package, Some(name.to_string())),
    };

    let optional = details.and_then(|it| it.optional).unwrap_or(false);
    let default_features = details.and_then(|it| it.default_features).unwrap_or(true);
    let features = details
        .and_then(|it| it.features.clone())
        .unwrap_or_default();
    let registry = details.and_then(|it| it.registry.clone());

    EncodableCrateDependency {
        name: crate_name,
        version_req: req,
        optional,
        default_features,
        features,
        target: target.map(ToString::to_string),
        kind: Some(kind),
        explicit_name_in_toml,
        registry,
    }
}

pub fn validate_dependency(dep: &EncodableCrateDependency) -> AppResult<()> {
    Crate::validate_crate_name("dependency", &dep.name).map_err(bad_request)?;

    for feature in &dep.features {
        Crate::validate_feature(feature).map_err(bad_request)?;
    }

    if let Some(registry) = &dep.registry {
        if !registry.is_empty() {
            return Err(bad_request(format_args!("Dependency `{}` is hosted on another registry. Cross-registry dependencies are not permitted on crates.io.", dep.name)));
        }
    }

    match semver::VersionReq::parse(&dep.version_req) {
        Err(_) => {
            return Err(bad_request(format_args!(
                "\"{}\" is an invalid version requirement",
                dep.version_req
            )));
        }
        Ok(req) if req == semver::VersionReq::STAR => {
            return Err(bad_request(format_args!("wildcard (`*`) dependency constraints are not allowed \
                on crates.io. Crate with this problem: `{}` See https://doc.rust-lang.org/cargo/faq.html#can-\
                libraries-use--as-a-version-for-their-dependencies for more \
                information", dep.name)));
        }
        _ => {}
    }

    if let Some(toml_name) = &dep.explicit_name_in_toml {
        Crate::validate_dependency_name(toml_name).map_err(bad_request)?;
    }

    Ok(())
}

#[instrument(skip_all)]
pub fn add_dependencies(
    conn: &mut impl Conn,
    deps: &[EncodableCrateDependency],
    version_id: i32,
) -> AppResult<()> {
    use diesel::insert_into;
    use diesel::RunQueryDsl;

    let crate_ids = crates::table
        .select((crates::name, crates::id))
        .filter(crates::name.eq_any(deps.iter().map(|d| &d.name)))
        .load_iter::<(String, i32), DefaultLoadingMode>(conn)?
        .collect::<QueryResult<HashMap<_, _>>>()?;

    let new_dependencies = deps
        .iter()
        .map(|dep| {
            // Match only identical names to ensure the index always references the original crate name
            let Some(&crate_id) = crate_ids.get(&dep.name) else {
                return Err(bad_request(format_args!(
                    "no known crate named `{}`",
                    dep.name
                )));
            };

            Ok((
                dependencies::version_id.eq(version_id),
                dependencies::crate_id.eq(crate_id),
                dependencies::req.eq(dep.version_req.to_string()),
                dependencies::kind.eq(dep.kind.unwrap_or(DependencyKind::Normal)),
                dependencies::optional.eq(dep.optional),
                dependencies::default_features.eq(dep.default_features),
                dependencies::features.eq(&dep.features),
                dependencies::target.eq(dep.target.as_deref()),
                dependencies::explicit_name.eq(dep.explicit_name_in_toml.as_deref()),
            ))
        })
        .collect::<Result<Vec<_>, _>>()?;

    insert_into(dependencies::table)
        .values(&new_dependencies)
        .execute(conn)?;

    Ok(())
}

impl From<TarballError> for BoxedAppError {
    fn from(error: TarballError) -> Self {
        match error {
            TarballError::Malformed(_err) => bad_request(
                "uploaded tarball is malformed or too large when decompressed",
            ),
            TarballError::InvalidPath(path) => bad_request(format!("invalid path found: {path}")),
            TarballError::UnexpectedSymlink(path) => {
                bad_request(format!("unexpected symlink or hard link found: {path}"))
            }
            TarballError::IO(err) => err.into(),
            TarballError::MissingManifest => {
                bad_request("uploaded tarball is missing a `Cargo.toml` manifest file")
            }
            TarballError::IncorrectlyCasedManifest(name) => {
                bad_request(format!(
                    "uploaded tarball is missing a `Cargo.toml` manifest file; `{name}` was found, but must be named `Cargo.toml` with that exact casing",
                    name = name.to_string_lossy(),
                ))
            }
            TarballError::TooManyManifests(paths) => {
                let paths = paths
                    .into_iter()
                    .map(|path| {
                        path.file_name()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned()
                    })
                    .collect::<Vec<_>>()
                    .join("`, `");
                bad_request(format!(
                    "uploaded tarball contains more than one `Cargo.toml` manifest file; found `{paths}`"
                ))
            }
            TarballError::InvalidManifest(err) => bad_request(format!(
                "failed to parse `Cargo.toml` manifest file\n\n{err}"
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{missing_metadata_error_message, validate_url};

    #[test]
    fn deny_relative_urls() {
        assert_err!(validate_url(Some("https:/example.com/home"), "homepage"));
    }

    #[test]
    fn missing_metadata_error_message_test() {
        assert_eq!(missing_metadata_error_message(&["a"]), "missing or empty metadata fields: a. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields");
        assert_eq!(missing_metadata_error_message(&["a", "b"]), "missing or empty metadata fields: a, b. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields");
        assert_eq!(missing_metadata_error_message(&["a", "b", "c"]), "missing or empty metadata fields: a, b, c. Please see https://doc.rust-lang.org/cargo/reference/manifest.html for more information on configuring these fields");
    }
}
