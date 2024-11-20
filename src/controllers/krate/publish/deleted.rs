use chrono::{DateTime, SecondsFormat, Utc};
use diesel_async::AsyncPgConnection;
use http::StatusCode;
use semver::Version;

use crate::{
    schema::deleted_crates,
    sql::canon_crate_name,
    util::{
        diesel::prelude::*,
        errors::{custom, AppResult},
    },
};

/// Checks the given crate name and version against the deleted crates table,
/// ensuring that the crate version is allowed to be published.
///
/// If the crate version cannot be published, a
/// [`crate::util::errors::BoxedAppError`] will be returned with a user-oriented
/// message suitable for being returned directly by a controller.
pub async fn validate(
    conn: &mut AsyncPgConnection,
    name: &str,
    version: &Version,
) -> AppResult<()> {
    use diesel_async::RunQueryDsl;

    // Since the same crate may have been deleted multiple times, we need to
    // calculate the most restrictive set of conditions that the crate version
    // being published must adhere to; specifically: the latest available_at
    // time, and the highest min_version.
    let mut state = State::default();

    // To do this, we need to iterate over all the relevant deleted crates.
    for (available_at, min_version) in deleted_crates::table
        .filter(canon_crate_name(deleted_crates::name).eq(canon_crate_name(name)))
        .select((deleted_crates::available_at, deleted_crates::min_version))
        .load::<(DateTime<Utc>, Option<String>)>(conn)
        .await?
    {
        state.observe_available_at(available_at);

        // We shouldn't really end up with an invalid semver in the
        // `min_version` field, so we're going to silently swallow any errors
        // for now.
        if let Some(Ok(min_version)) = min_version.map(|v| Version::parse(&v)) {
            state.observe_min_version(min_version);
        }
    }

    // Finally, we can check the given name and version against the built up
    // state and see if it passes.
    state.into_result(name, version, Utc::now())
}

#[derive(Default)]
#[cfg_attr(test, derive(Clone))]
struct State {
    available_at: Option<DateTime<Utc>>,
    min_version: Option<Version>,
}

impl State {
    fn observe_available_at(&mut self, available_at: DateTime<Utc>) {
        if let Some(current) = self.available_at {
            self.available_at = Some(std::cmp::max(current, available_at));
        } else {
            self.available_at = Some(available_at);
        }
    }

    fn observe_min_version(&mut self, min_version: Version) {
        if let Some(current) = self.min_version.take() {
            self.min_version = Some(std::cmp::max(current, min_version));
        } else {
            self.min_version = Some(min_version);
        }
    }

    fn into_result(self, name: &str, version: &Version, now: DateTime<Utc>) -> AppResult<()> {
        let mut messages = Vec::new();

        if let Some(available_at) = self.available_at {
            if now < available_at {
                messages.push(format!(
                    "Reuse of this name will be available after {}.",
                    available_at.to_rfc3339_opts(SecondsFormat::Secs, true)
                ));
            }
        }

        if let Some(min_version) = self.min_version {
            if version < &min_version {
                messages.push(format!("To avoid conflicts with previously published versions of this crate, the minimum version that can be published is {min_version}."));
            }
        }

        if messages.is_empty() {
            Ok(())
        } else {
            Err(custom(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!(
                    "A crate with the name `{name}` was previously deleted.\n\n* {}",
                    messages.join("\n* "),
                ),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeDelta;
    use insta::assert_snapshot;

    use super::*;

    macro_rules! assert_result_status {
        ($result:expr) => {{
            let response = $result.unwrap_err().response();
            assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

            String::from_utf8(
                axum::body::to_bytes(response.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .into(),
            )
            .unwrap()
        }};
    }

    macro_rules! assert_result_failed {
        ($result:expr) => {{
            let text = assert_result_status!($result);
            assert_snapshot!(text);
        }};
        ($result:expr, $name:tt) => {{
            let text = assert_result_status!($result);
            assert_snapshot!($name, text);
        }};
    }

    #[test]
    fn empty_state() {
        let state = State::default();

        // Any combination of values should result in Ok, since there are no
        // deleted crates.
        for (name, version, now) in [
            ("foo", "0.0.0", "2024-11-20T01:00:00Z"),
            ("bar", "1.0.0", "1970-01-01T00:00:00Z"),
        ] {
            assert_ok!(state.clone().into_result(
                name,
                &Version::parse(version).unwrap(),
                now.parse().unwrap()
            ));
        }
    }

    #[tokio::test]
    async fn available_at_only() {
        let available_at = "2024-11-20T00:00:00Z".parse().unwrap();
        let version = Version::parse("0.0.0").unwrap();

        let mut state = State::default();
        state.observe_available_at(available_at);

        // There should be no error for a crate after available_at.
        assert_ok!(state.clone().into_result(
            "foo",
            &version,
            available_at + TimeDelta::seconds(60)
        ));

        // Similarly, a crate actually _at_ available_at should be fine.
        assert_ok!(state.clone().into_result("foo", &version, available_at));

        // But a crate one second earlier should error.
        assert_result_failed!(state.into_result(
            "foo",
            &version,
            available_at - TimeDelta::seconds(1)
        ));
    }

    #[tokio::test]
    async fn min_version_only() {
        let available_at = "2024-11-20T00:00:00Z".parse().unwrap();

        let mut state = State::default();
        state.observe_available_at(available_at);

        // Test the versions that we expect to pass.
        for (min_version, publish_version) in [
            ("0.1.0", "0.1.0"),
            ("0.1.0", "0.1.1"),
            ("0.1.0", "0.2.0"),
            ("0.1.0", "1.0.0"),
            ("1.0.0", "1.0.0"),
            ("1.0.0", "1.0.1"),
            ("1.0.0", "2.0.0"),
        ] {
            let mut state = state.clone();
            state.observe_min_version(Version::parse(min_version).unwrap());

            assert_ok!(state.into_result(
                "foo",
                &Version::parse(publish_version).unwrap(),
                available_at
            ));
        }

        // Now test the versions that we expect to fail.
        for (min_version, publish_version) in [("0.1.0", "0.0.0"), ("1.0.0", "0.1.0")] {
            let mut state = state.clone();
            state.observe_min_version(Version::parse(min_version).unwrap());

            assert_result_failed!(
                state.into_result(
                    "foo",
                    &Version::parse(publish_version).unwrap(),
                    available_at,
                ),
                publish_version
            );
        }
    }

    #[tokio::test]
    async fn multiple_deleted() {
        // We won't repeat everything from the above here, but let's make sure
        // the most restrictive available_at and min_version are used when
        // multiple deleted crates are observed.
        let mut state = State::default();

        let earlier_available_at = "2024-11-20T00:00:00Z".parse().unwrap();
        let later_available_at = "2024-11-21T12:00:00Z".parse().unwrap();
        state.observe_available_at(earlier_available_at);
        state.observe_available_at(later_available_at);
        state.observe_available_at(earlier_available_at);

        let first_version = Version::parse("0.1.0").unwrap();
        let second_version = Version::parse("1.0.0").unwrap();
        state.observe_min_version(first_version.clone());
        state.observe_min_version(second_version.clone());
        state.observe_min_version(first_version.clone());

        assert_ok!(state
            .clone()
            .into_result("foo", &second_version, later_available_at));

        // Now the bad cases.
        for (name, version, now) in [
            ("min_version", &first_version, later_available_at),
            ("available_at", &second_version, earlier_available_at),
            ("both", &first_version, earlier_available_at),
        ] {
            assert_result_failed!(state.clone().into_result("foo", version, now), name);
        }
    }
}
