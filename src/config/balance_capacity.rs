use crates_io_env_vars::var_parsed;
use std::env;

pub struct BalanceCapacityConfig {
    pub report_only: bool,
    pub log_total_at_count: usize,
    pub log_at_percentage: usize,
    pub throttle_at_percentage: usize,
    pub dl_only_at_percentage: usize,
}

impl BalanceCapacityConfig {
    pub fn from_environment() -> anyhow::Result<Self> {
        Ok(Self {
            report_only: env::var("WEB_CAPACITY_REPORT_ONLY").is_ok(),
            log_total_at_count: var_parsed("WEB_CAPACITY_LOG_TOTAL_AT_COUNT")?.unwrap_or(50),
            // The following are a percentage of `db_capacity`
            log_at_percentage: var_parsed("WEB_CAPACITY_LOG_PCT")?.unwrap_or(50),
            throttle_at_percentage: var_parsed("WEB_CAPACITY_THROTTLE_PCT")?.unwrap_or(70),
            dl_only_at_percentage: var_parsed("WEB_CAPACITY_DL_ONLY_PCT")?.unwrap_or(80),
        })
    }
}
