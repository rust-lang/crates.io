//! Configuration for setting up database pools
//!
//! - `DATABASE_URL`: The URL of the postgres database to use.
//! - `READ_ONLY_REPLICA_URL`: The URL of an optional postgres read-only replica database.
//! - `DB_OFFLINE`: If set to `leader` then use the read-only follower as if it was the leader.
//!   If set to `follower` then act as if `READ_ONLY_REPLICA_URL` was unset.
//! - `READ_ONLY_MODE`: If defined (even as empty) then force all connections to be read-only.

use crate::env;

pub struct DatabasePools {
    /// Settings for the primary database. This is usually writeable, but will be read-only in
    /// some configurations.
    pub primary: DbPoolConfig,
    /// An optional follower database. Always read-only.
    pub replica: Option<DbPoolConfig>,
}

#[derive(Debug)]
pub struct DbPoolConfig {
    pub url: String,
    pub read_only_mode: bool,
}

impl DatabasePools {
    pub fn are_all_read_only(&self) -> bool {
        self.primary.read_only_mode
    }
}

impl DatabasePools {
    /// Load settings for one or more database pools from the environment
    ///
    /// # Panics
    ///
    /// This function panics if `DB_OFFLINE=leader` but `READ_ONLY_REPLICA_URL` is unset.
    pub fn full_from_environment() -> Self {
        let leader_url = env("DATABASE_URL");
        let follower_url = dotenv::var("READ_ONLY_REPLICA_URL").ok();
        let read_only_mode = dotenv::var("READ_ONLY_MODE").is_ok();
        match dotenv::var("DB_OFFLINE").as_deref() {
            // The actual leader is down, use the follower in read-only mode as the primary and
            // don't configure a replica.
            Ok("leader") => Self {
                primary: DbPoolConfig {
                    url: follower_url
                        .expect("Must set `READ_ONLY_REPLICA_URL` when using `DB_OFFLINE=leader`."),
                    read_only_mode: true,
                },
                replica: None,
            },
            // The follower is down, don't configure the replica.
            Ok("follower") => Self {
                primary: DbPoolConfig {
                    url: leader_url,
                    read_only_mode,
                },
                replica: None,
            },
            _ => Self {
                primary: DbPoolConfig {
                    url: leader_url,
                    read_only_mode,
                },
                replica: follower_url.map(|url| DbPoolConfig {
                    url,
                    // Always enable read-only mode for the follower. In staging, we attach the
                    // same leader database to both environment variables and this ensures the
                    // connection is opened read-only even when attached to a writeable database.
                    read_only_mode: true,
                }),
            },
        }
    }

    pub fn test_from_environment() -> Self {
        DatabasePools {
            primary: DbPoolConfig {
                url: env("TEST_DATABASE_URL"),
                read_only_mode: false,
            },
            replica: None,
        }
    }
}
