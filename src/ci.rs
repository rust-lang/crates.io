use crate::middleware::real_ip::RealIp;
use async_trait::async_trait;
use axum::extract::FromRequestParts;
use http::request::Parts;
use ipnetwork::IpNetwork;
use std::fmt::Display;
use std::net::IpAddr;
use std::sync::LazyLock;

#[derive(Copy, Clone, Debug)]
pub enum CiService {
    GitHubActions,
}

impl Display for CiService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CiService::GitHubActions => write!(f, "GitHub Actions"),
        }
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for CiService {
    type Rejection = ();

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let real_ip = parts.extensions.get::<RealIp>().ok_or(())?;

        if is_github_actions_ip(real_ip) {
            return Ok(CiService::GitHubActions);
        }

        Err(())
    }
}

fn is_github_actions_ip(ip: &IpAddr) -> bool {
    static GITHUB_ACTIONS_CIDRS: LazyLock<Vec<IpNetwork>> = LazyLock::new(|| {
        github_meta::META
            .actions
            .iter()
            .filter_map(|cidr| parse_cidr(cidr, "GitHub Actions"))
            .collect()
    });

    GITHUB_ACTIONS_CIDRS
        .iter()
        .any(|trusted_proxy| trusted_proxy.contains(*ip))
}

fn parse_cidr(cidr: &str, service: &'static str) -> Option<IpNetwork> {
    match cidr.parse() {
        Ok(ip_network) => Some(ip_network),
        Err(error) => {
            warn!(%cidr, %error, "Failed to parse {service} CIDR");
            None
        }
    }
}
