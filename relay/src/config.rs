//! Environment-backed configuration for the relay service.

use std::{env, net::SocketAddr};

use thiserror::Error;

/// Runtime settings loaded from the process environment.
#[derive(Clone)]
pub struct Settings {
    pub bind_addr: SocketAddr,
    pub auth_token: String,
    pub adguard_base_url: String,
    pub adguard_username: Option<String>,
    pub adguard_password: Option<String>,
    pub allowed_ttl_secs: u64,
    pub blocked_ttl_secs: u64,
}

impl Settings {
    /// Loads all configuration values from environment variables, applying only
    /// safe operational defaults for non-secret fields.
    pub fn from_env() -> Result<Self, ConfigError> {
        Ok(Self {
            bind_addr: read_var("RELAY_BIND_ADDR")
                .unwrap_or_else(|_| "0.0.0.0:8080".to_owned())
                .parse()
                .map_err(ConfigError::InvalidBindAddr)?,
            auth_token: read_var("RELAY_AUTH_TOKEN")?,
            adguard_base_url: read_var("ADGUARD_BASE_URL")
                .unwrap_or_else(|_| "http://adguardhome:3000".to_owned()),
            adguard_username: read_optional_var("ADGUARD_USERNAME"),
            adguard_password: read_optional_var("ADGUARD_PASSWORD"),
            allowed_ttl_secs: read_u64("RELAY_ALLOWED_TTL_SECS", 3600)?,
            blocked_ttl_secs: read_u64("RELAY_BLOCKED_TTL_SECS", 86_400)?,
        })
    }
}

/// Reads a required string environment variable.
fn read_var(name: &'static str) -> Result<String, ConfigError> {
    env::var(name).map_err(|_| ConfigError::MissingVar(name))
}

/// Reads an optional string environment variable and treats empty values as unset.
fn read_optional_var(name: &'static str) -> Option<String> {
    env::var(name).ok().filter(|value| !value.is_empty())
}

/// Reads an optional numeric environment variable with a default fallback.
fn read_u64(name: &'static str, default: u64) -> Result<u64, ConfigError> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u64>()
            .map_err(|_| ConfigError::InvalidNumber(name, value)),
        Err(_) => Ok(default),
    }
}

/// Configuration errors surfaced during process startup.
#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("missing required environment variable {0}")]
    MissingVar(&'static str),
    #[error("invalid bind address: {0}")]
    InvalidBindAddr(std::net::AddrParseError),
    #[error("invalid numeric value for {0}: {1}")]
    InvalidNumber(&'static str, String),
}