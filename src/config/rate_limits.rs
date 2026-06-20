use std::collections::HashMap;
use std::time::Duration;

use crates_io_env_vars::var_parsed;

use crate::rate_limiter::{LimitedAction, RateLimiterConfig};

#[derive(Debug, Default)]
pub struct RateLimitsConfig {
    /// Per-action rate limiter configuration, keyed by [`LimitedAction`].
    ///
    /// Loaded from the `RATE_LIMITER_{ACTION}_RATE_SECONDS` and
    /// `RATE_LIMITER_{ACTION}_BURST` environment variables, falling back to
    /// each action's defaults.
    pub actions: HashMap<LimitedAction, RateLimiterConfig>,

    /// Maximum number of new versions a user can publish per day.
    ///
    /// Read from the `MAX_NEW_VERSIONS_DAILY` environment variable.
    pub new_versions_daily: Option<u32>,
}

impl RateLimitsConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        // Dynamically load the configuration for all the rate limiting actions. See
        // `src/rate_limiter.rs` for their definition.
        let mut actions = HashMap::new();
        for action in LimitedAction::VARIANTS {
            let env_var_key = action.env_var_key();
            actions.insert(
                *action,
                RateLimiterConfig {
                    rate: Duration::from_secs(
                        var_parsed(&format!("RATE_LIMITER_{env_var_key}_RATE_SECONDS"))?
                            .unwrap_or_else(|| action.default_rate_seconds()),
                    ),
                    burst: var_parsed(&format!("RATE_LIMITER_{env_var_key}_BURST"))?
                        .unwrap_or_else(|| action.default_burst()),
                },
            );
        }

        let new_versions_daily = var_parsed("MAX_NEW_VERSIONS_DAILY")?;

        Ok(Self {
            actions,
            new_versions_daily,
        })
    }
}
