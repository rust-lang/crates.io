use url::Url;

/// Hosts in this list are known to not be hosting documentation,
/// and are possibly of malicious intent e.g. ad tracking networks, etc.
const DOMAIN_BLOCKLIST: &[&str] = &[
    "rust-ci.org",
    "rustless.org",
    "ironframework.io",
    "nebulanet.cc",
];

/// Return `None` if the documentation URL host matches a blocked host
pub fn remove_blocked_urls(url: Option<String>) -> Option<String> {
    // Handles if documentation URL is None
    let url = url?;

    // Handles unsuccessful parsing of documentation URL
    let parsed_url = match Url::parse(&url) {
        Ok(parsed_url) => parsed_url,
        Err(_) => return None,
    };

    // Extract host string from documentation URL
    let url_host = parsed_url.host_str()?;

    // Match documentation URL host against blocked host array elements
    if domain_is_blocked(url_host) {
        None
    } else {
        Some(url)
    }
}

fn domain_is_blocked(domain: &str) -> bool {
    DOMAIN_BLOCKLIST
        .iter()
        .any(|blocked| &domain == blocked || domain_is_subdomain(domain, blocked))
}

fn domain_is_subdomain(potential_subdomain: &str, root: &str) -> bool {
    if !potential_subdomain.ends_with(root) {
        return false;
    }

    let root_with_prefix = format!(".{root}");
    potential_subdomain.ends_with(&root_with_prefix)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_blocked_no_url_provided() {
        assert_eq!(remove_blocked_urls(None), None);
    }

    #[test]
    fn domain_blocked_invalid_url() {
        assert_eq!(remove_blocked_urls(Some(String::from("not a url"))), None);
    }

    #[test]
    fn domain_blocked_url_contains_partial_match() {
        assert_eq!(
            remove_blocked_urls(Some(String::from("http://rust-ci.organists.com")),),
            Some(String::from("http://rust-ci.organists.com"))
        );
    }

    #[test]
    fn domain_blocked_url() {
        assert_eq!(
            remove_blocked_urls(Some(String::from(
                "http://rust-ci.org/crate/crate-0.1/doc/crate-0.1",
            ),),),
            None
        );
    }

    #[test]
    fn domain_blocked_subdomain() {
        assert_eq!(
            remove_blocked_urls(Some(String::from(
                "http://www.rust-ci.org/crate/crate-0.1/doc/crate-0.1",
            ),),),
            None
        );
    }

    #[test]
    fn domain_blocked_non_subdomain() {
        let input = Some(String::from("http://foorust-ci.org/"));
        let result = remove_blocked_urls(input);
        assert_some_eq!(result, "http://foorust-ci.org/");
    }
}
