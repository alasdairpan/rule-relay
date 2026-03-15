//! Small client wrapper around the subset of AdGuard Home APIs used by the relay.

use std::time::Duration;

use reqwest::StatusCode;
use serde::Deserialize;
use thiserror::Error;

/// Thin client for AdGuard Home's `check_host` endpoint.
#[derive(Clone)]
pub struct AdguardClient {
    base_url: String,
    username: Option<String>,
    password: Option<String>,
    client: reqwest::Client,
}

impl AdguardClient {
    /// Creates a new client that talks to the configured AdGuard base URL.
    pub fn new(base_url: String, username: Option<String>, password: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .expect("reqwest client should build");

        Self {
            base_url: base_url.trim_end_matches('/').to_owned(),
            username,
            password,
            client,
        }
    }

    /// Checks a hostname against AdGuard Home's filtering decision endpoint.
    pub async fn check_host(&self, domain: &str) -> Result<AdguardCheckHostResponse, AdguardError> {
        let url = format!("{}/control/filtering/check_host", self.base_url);

        let request = self.client.get(url).query(&[("name", domain)]);

        let request = match (&self.username, &self.password) {
            (Some(username), Some(password)) => request.basic_auth(username, Some(password)),
            _ => request,
        };

        let response = request.send().await.map_err(AdguardError::Transport)?;

        if response.status() != StatusCode::OK {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            tracing::error!(status = %status, body, "adguard returned unexpected response");

            return Err(AdguardError::UnexpectedStatus { status });
        }

        response
            .json::<AdguardCheckHostResponse>()
            .await
            .map_err(AdguardError::Decode)
    }
}

/// Raw response payload returned by AdGuard Home.
#[derive(Debug, Deserialize)]
pub struct AdguardCheckHostResponse {
    pub reason: String,
    #[serde(default)]
    pub rule: String,
    #[serde(default)]
    pub rules: Vec<AdguardMatchedRule>,
    #[serde(default)]
    pub service_name: String,
    #[serde(default)]
    pub cname: String,
    #[serde(default, deserialize_with = "deserialize_string_vec_or_null")]
    pub ip_addrs: Vec<String>,
}

fn deserialize_string_vec_or_null<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

/// Individual matched rule entry returned by AdGuard Home when a rule fired.
#[derive(Debug, Deserialize)]
pub struct AdguardMatchedRule {
    pub text: String,
    #[serde(default)]
    pub filter_list_id: u64,
}

/// Internal error type for AdGuard transport and decode failures.
#[derive(Debug, Error)]
pub enum AdguardError {
    #[error("failed to reach adguard: {0}")]
    Transport(reqwest::Error),
    #[error("adguard returned unexpected status {status}")]
    UnexpectedStatus { status: StatusCode },
    #[error("failed to decode adguard response: {0}")]
    Decode(reqwest::Error),
}