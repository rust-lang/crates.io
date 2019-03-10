use conduit::Request;
use std::collections::HashMap;

use crate::db::{DieselPool};
use crate::middleware::app::RequestApp;
use crate::models::{User};
use crate::util::errors::{CargoResult, TooManyRequests};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Duration, Utc};

/// Settings for a rate-limited route.
#[derive(Debug, Clone)]
pub struct RateLimitSettings {
    /// The code for this category. Can be stored in a database, etc.
    pub key: String,
    /// The maximum number of tokens that can be acquired.
    pub max_amount: usize,
    /// How often we refill
    pub refill_time: Duration,
    /// The number of tokens that are added during a refill.
    pub refill_amount: usize,
}

/// The result from a rate limit check.
#[derive(Debug, Clone, Copy)]
pub struct RateLimitResult {
    /// The remaining number of requests available
    remaining: usize,
}

/// A type that can perform rate limiting.
pub trait RateLimiter {
    fn check_limit_multiple(&self, tokens: u32, user: &User, category: RateLimitCategory) -> CargoResult<RateLimitResult>;
}

/// Rate limit using a postgresql database.
#[allow(missing_debug_implementations)]
#[derive(Clone)]
pub struct RateLimiterPostgres {
    diesel_database: DieselPool,
}

impl RateLimiterPostgres {
    /// Create a new postgres rate limiter from the given database pool.
    pub fn new(diesel_database: DieselPool) -> RateLimiterPostgres {
        RateLimiterPostgres {
            diesel_database,
        }
    }
}

impl RateLimiter for RateLimiterPostgres {
    fn check_limit_multiple(&self, _tokens: u32, _user: &User, _category: RateLimitCategory) -> CargoResult<RateLimitResult> {
        let _conn = self.diesel_database.get()?;

        // TODO: Database interaction.
        Err(Box::new(TooManyRequests))
    }
}

type UserId = i32;
type RateLimiterMemoryKey = (UserId, RateLimitCategory);
#[derive(Debug, Clone, Copy)]
struct RateLimiterMemoryValue {
    pub value: usize,
    // TODO: Time.
    pub last_update: DateTime<Utc>,
}

/// Rate limit using an internal memory store. This may not be ideal in a load-balanced
/// environment, unless all requests from a user get routed to the same instance.
#[derive(Debug, Clone)]
pub struct RateLimiterMemory {
    data: Arc<Mutex<HashMap<RateLimiterMemoryKey, RateLimiterMemoryValue>>>,
}

impl RateLimiterMemory {
    /// Create a new memory-based rate limiter
    pub fn new() -> RateLimiterMemory {
        RateLimiterMemory {
            data: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl RateLimiter for RateLimiterMemory {
    fn check_limit_multiple(&self, tokens: u32, user: &User, category: RateLimitCategory) -> CargoResult<RateLimitResult> {
        let mut data = self.data.lock().unwrap();
        let settings = category.settings();
        let now = Utc::now();
        let mut entry = data.entry((user.id, category)).or_insert_with(|| RateLimiterMemoryValue { value: settings.max_amount, last_update: now });
        println!("Previous entry: {:?}", entry);
        let mut now2 = now;
        let mut refill_count = 0;
        while now2 > entry.last_update + settings.refill_time {
            now2 = now2 - settings.refill_time;
            refill_count += 1;
        }
        entry.value = std::cmp::min(
            settings.max_amount,
            entry.value + refill_count * settings.refill_amount);

        entry.last_update = std::cmp::min(
            now,
            entry.last_update + (settings.refill_time * refill_count as i32));

        if entry.value < tokens as usize {
            return Err(Box::new(TooManyRequests));
        }

        entry.value -= tokens as usize;

        Ok(RateLimitResult { remaining: entry.value })
    }
}

/// A rate limiter that does not limit at all.
#[derive(Debug, Clone, Copy)]
pub struct RateLimiterUnlimited;

impl RateLimiter for RateLimiterUnlimited {
    fn check_limit_multiple(&self, _tokens: u32, _user: &User, category: RateLimitCategory) -> CargoResult<RateLimitResult> {
        println!("Unlimited rate limiter!");
        let settings = category.settings();
        Ok(RateLimitResult { remaining: settings.max_amount })
    }
}

/// All of the possible rate limit buckets. When rate limiting a new endpoint, add it here and set
/// the settings below.
#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum RateLimitCategory {
    PublishCrate,
    // How often a new crate can be uploaded
    NewCrate,
    // How often an uploaded crate can be uploaded
    NewVersion,
    // How often a request for crate info can be made
    CrateInfo,
}

impl RateLimitCategory {
    pub fn settings(&self) -> RateLimitSettings {
        use RateLimitCategory::*;
        match *self {
            PublishCrate => RateLimitSettings { key: "publish-crate".into(), max_amount: 3, refill_time: Duration::seconds(60), refill_amount: 1 },
            NewCrate => RateLimitSettings { key: "new-crate".into(), max_amount: 3, refill_time: Duration::seconds(60), refill_amount: 1 },
            NewVersion => RateLimitSettings { key: "new_version".into(), max_amount: 60, refill_time: Duration::seconds(1), refill_amount: 1 },
            CrateInfo => RateLimitSettings { key: "crate-info".into(), max_amount: 5, refill_time: Duration::seconds(10), refill_amount: 1 },
        }
    }
}

/// A trait that makes it possible to call `check_rate_limit` directly on a request object.
pub trait RequestRateLimit {
    /// Check the rate limit for the given endpoint. This function consumes a single token from the
    /// token bucket.
    fn check_rate_limit(&mut self, user: &User, category: RateLimitCategory) -> CargoResult<RateLimitResult>;
}

impl<T: Request + ?Sized> RequestRateLimit for T {
    fn check_rate_limit(&mut self, user: &User, category: RateLimitCategory) -> CargoResult<RateLimitResult> {
        let limiter = &self.app().rate_limiter;
        limiter.check_limit_multiple(1, user, category)
    }
}
