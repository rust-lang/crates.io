/// Determines if downloads from the given user agent should be counted.
///
/// Returns `true` if the download should be counted, `false` otherwise.
pub fn should_count_user_agent(user_agent: &str) -> bool {
    let Some(suffix) = user_agent.strip_prefix("cargo") else {
        return false;
    };

    suffix.starts_with('/')
        || suffix.starts_with(' ')
        || suffix.starts_with("%2f")
        || suffix.starts_with("%2F")
        || suffix.starts_with("%20")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_should_count_user_agent() {
        // Standard cargo user agents with forward slash
        assert!(should_count_user_agent(
            "cargo/1.92.0-nightly (344c4567c 2025-10-21)"
        ));
        assert!(should_count_user_agent(
            "cargo/1.88.0 (873a06493 2025-05-10)"
        ));
        assert!(should_count_user_agent(
            "cargo/1.90.0 (840b83a10 2025-07-30)"
        ));
        assert!(should_count_user_agent("cargo/"));

        // CloudFront: Percent-encoded forward slash (lowercase and uppercase)
        assert!(should_count_user_agent("cargo%2f1.74.0"));
        assert!(should_count_user_agent("cargo%2F1.74.0"));

        // Space character (legacy Cargo versions)
        assert!(should_count_user_agent("cargo 1.74.0"));

        // CloudFront: Percent-encoded space (legacy Cargo versions)
        assert!(should_count_user_agent(
            "cargo%201.74.0%20(ecb9851af%202023-10-18)"
        ));
        assert!(should_count_user_agent("cargo%20"));

        // Non-cargo user agents
        assert!(!should_count_user_agent("Mozilla/5.0"));
        assert!(!should_count_user_agent("curl/7.64.1"));
        assert!(!should_count_user_agent(""));
        assert!(!should_count_user_agent("Cargo/1.0.0"));
        assert!(!should_count_user_agent("cargo"));
        assert!(!should_count_user_agent("cargo-"));
    }
}
