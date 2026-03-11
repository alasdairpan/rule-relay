//! Hostname normalization and validation helpers.

use thiserror::Error;
use url::Host;

/// Normalizes a caller-supplied hostname into the canonical cache and lookup key.
pub fn normalize_domain(input: &str) -> Result<String, DomainError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(DomainError::Empty);
    }

    if trimmed.chars().any(char::is_whitespace) {
        return Err(DomainError::ContainsWhitespace);
    }

    if trimmed.contains("//") || trimmed.contains('/') || trimmed.contains('?') || trimmed.contains('#') {
        return Err(DomainError::ExpectedHostname);
    }

    let normalized = trimmed.trim_end_matches('.').to_ascii_lowercase();
    if normalized.is_empty() {
        return Err(DomainError::Empty);
    }

    match Host::parse(&normalized).map_err(|_| DomainError::InvalidHostname)? {
        Host::Domain(domain) => Ok(domain.to_owned()),
        Host::Ipv4(_) | Host::Ipv6(_) => Err(DomainError::IpAddressNotAllowed),
    }
}

/// Validation errors for client-supplied domain values.
#[derive(Debug, Error)]
pub enum DomainError {
    #[error("domain is required")]
    Empty,
    #[error("domain must not contain whitespace")]
    ContainsWhitespace,
    #[error("domain must be a hostname without scheme or path")]
    ExpectedHostname,
    #[error("invalid hostname")]
    InvalidHostname,
    #[error("ip literals are not supported")]
    IpAddressNotAllowed,
}

#[cfg(test)]
mod tests {
    use super::normalize_domain;

    #[test]
    fn normalizes_hostname() {
        assert_eq!(normalize_domain("WWW.Example.COM.").unwrap(), "www.example.com");
    }

    #[test]
    fn rejects_url() {
        assert!(normalize_domain("https://example.com").is_err());
    }

    #[test]
    fn rejects_ip_literal() {
        assert!(normalize_domain("1.1.1.1").is_err());
    }
}