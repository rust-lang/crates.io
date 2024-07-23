use std::sync::Arc;

use crate::util::diesel::Conn;
use thiserror::Error;
use typomania::{
    checks::{Bitflips, Omitted, SwappedWords, Typos},
    Harness,
};

use super::{checks::Affixes, config, database::TopCrates};

static NOTIFICATION_EMAILS_ENV: &str = "TYPOSQUAT_NOTIFICATION_EMAILS";

/// A cache containing everything we need to run typosquatting checks.
///
/// Specifically, this includes a corpus of popular crates attached to a typomania harness, and a
/// list of e-mail addresses that we'll send notifications to if potential typosquatting is
/// discovered.
pub struct Cache {
    emails: Vec<String>,
    harness: Option<Harness<TopCrates>>,
}

impl Cache {
    /// Instantiates a new [`Cache`] from the environment.
    ///
    /// This reads the `NOTIFICATION_EMAILS_ENV` environment variable to get the list of e-mail
    /// addresses to send notifications to, then invokes [`Cache::new`] to read popular crates from
    /// the database.
    #[instrument(skip_all, err)]
    pub fn from_env(conn: &mut impl Conn) -> Result<Self, Error> {
        let emails: Vec<String> = crates_io_env_vars::var(NOTIFICATION_EMAILS_ENV)
            .map_err(|e| Error::Environment {
                name: NOTIFICATION_EMAILS_ENV.into(),
                source: Arc::new(e),
            })?
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_owned())
            .filter(|s| !s.is_empty())
            .collect();

        if emails.is_empty() {
            // If we're not notifying anyone, then there's really not much to do here.
            warn!("$TYPOSQUAT_NOTIFICATION_EMAILS is not set; no typosquatting notifications will be sent");
            Ok(Self {
                emails,
                harness: None,
            })
        } else {
            // Otherwise, let's go get the top crates and build a corpus.
            Self::new(emails, conn)
        }
    }

    /// Instantiates a cache by querying popular crates and building them into a typomania harness.
    ///
    /// This relies on configuration in the `super::config` module.
    pub fn new(emails: Vec<String>, conn: &mut impl Conn) -> Result<Self, Error> {
        let top = TopCrates::new(conn, config::TOP_CRATES)?;

        Ok(Self {
            emails,
            harness: Some(
                Harness::builder()
                    .with_check(Bitflips::new(
                        config::CRATE_NAME_ALPHABET,
                        top.crates.keys().map(String::as_str),
                    ))
                    .with_check(Omitted::new(config::CRATE_NAME_ALPHABET))
                    .with_check(SwappedWords::new("-_"))
                    .with_check(Typos::new(config::TYPOS.iter().map(|(c, typos)| {
                        (*c, typos.iter().map(|ss| ss.to_string()).collect())
                    })))
                    .with_check(Affixes::new(
                        config::SUFFIXES.iter(),
                        config::SUFFIX_SEPARATORS.iter(),
                    ))
                    .build(top),
            ),
        })
    }

    pub fn get_harness(&self) -> Option<&Harness<TopCrates>> {
        self.harness.as_ref()
    }

    pub fn iter_emails(&self) -> impl Iterator<Item = &str> {
        self.emails.iter().map(String::as_str)
    }
}

// Because the error returned from Cache::new() gets memoised in the environment, we either need to
// return it by reference from Environment::typosquat_cache() or we need to be able to clone it.
// We'll do some Arc wrapping in the variants below to ensure that everything is clonable while not
// destroying the source metadata.
#[derive(Error, Debug, Clone)]
pub enum Error {
    #[error("error reading environment variable {name}: {source:?}")]
    Environment {
        name: String,
        #[source]
        source: Arc<anyhow::Error>,
    },

    #[error("error getting top crates: {0:?}")]
    TopCrates(#[source] Arc<diesel::result::Error>),
}

impl From<diesel::result::Error> for Error {
    fn from(value: diesel::result::Error) -> Self {
        Self::TopCrates(Arc::new(value))
    }
}
