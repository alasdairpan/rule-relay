//! Request and response models exposed by the relay API.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::adguard::AdguardCheckHostResponse;

/// Query parameters accepted by the `GET /v1/domain-check` route.
#[derive(Debug, Deserialize)]
pub struct DomainCheckQuery {
    pub domain: String,
}

/// Normalized response returned to clients after a cache or AdGuard lookup.
#[derive(Clone, Debug, Serialize)]
pub struct DomainCheckResponse {
    pub domain: String,
    pub blocked: bool,
    pub reason: DecisionReason,
    pub ttl: u64,
    pub checked_at: DateTime<Utc>,
    pub source: &'static str,
    pub cache_status: CacheStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service_name: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rules: Vec<MatchedRule>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cname: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ip_addrs: Vec<String>,
}

impl DomainCheckResponse {
    /// Converts the raw AdGuard response into the stable public relay schema.
    pub fn from_adguard(
        domain: String,
        upstream: AdguardCheckHostResponse,
        ttl: u64,
        cache_status: CacheStatus,
        checked_at: DateTime<Utc>,
    ) -> Self {
        let rule = upstream
            .rules
            .first()
            .map(|entry| entry.text.clone())
            .or_else(|| (!upstream.rule.is_empty()).then_some(upstream.rule.clone()));
        let rules = upstream
            .rules
            .into_iter()
            .map(|entry| MatchedRule {
                text: entry.text,
                filter_list_id: (entry.filter_list_id != 0).then_some(entry.filter_list_id),
            })
            .collect::<Vec<_>>();
        let service_name = (!upstream.service_name.is_empty()).then_some(upstream.service_name.clone());
        let cname = (!upstream.cname.is_empty()).then_some(upstream.cname.clone());
        let reason = DecisionReason::from_adguard_reason(&upstream.reason);

        Self {
            domain,
            blocked: reason.is_blocked(),
            reason,
            ttl,
            checked_at,
            source: "adguard",
            cache_status,
            raw_reason: (!upstream.reason.is_empty()).then_some(upstream.reason),
            rule,
            service_name,
            rules,
            cname,
            ip_addrs: upstream.ip_addrs,
        }
    }

    /// Re-labels the cache status when an existing response is served from cache.
    pub fn with_cache_status(mut self, cache_status: CacheStatus) -> Self {
        self.cache_status = cache_status;
        self
    }
}

/// Matched filter rule metadata forwarded from AdGuard when available.
#[derive(Clone, Debug, Serialize)]
pub struct MatchedRule {
    pub text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_list_id: Option<i64>,
}

/// Stable reason categories exposed by the public relay API.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DecisionReason {
    BlockedAdult,
    BlockedRule,
    BlockedService,
    Allowed,
    AllowedAllowlist,
    Rewritten,
    Unknown,
}

impl DecisionReason {
    /// Maps AdGuard-specific reason strings into a smaller public enum.
    pub fn from_adguard_reason(reason: &str) -> Self {
        match reason {
            "FilteredParental" => Self::BlockedAdult,
            "FilteredBlockedService" => Self::BlockedService,
            "NotFilteredAllowList" => Self::AllowedAllowlist,
            "NotFilteredNotFound" => Self::Allowed,
            "Rewrite" | "RewriteEtcHosts" | "RewriteRule" | "Rewritten" => Self::Rewritten,
            _ if reason.starts_with("Filtered") => Self::BlockedRule,
            _ if reason.starts_with("NotFiltered") => Self::Allowed,
            _ => Self::Unknown,
        }
    }

    pub fn is_blocked(self) -> bool {
        matches!(self, Self::BlockedAdult | Self::BlockedRule | Self::BlockedService)
    }
}

/// Convenience helper used by the request pipeline when only the blocked bit is needed.
pub fn is_blocked_reason(reason: &str) -> bool {
    DecisionReason::from_adguard_reason(reason).is_blocked()
}

/// Indicates whether a response came from the in-memory cache or a live lookup.
#[derive(Clone, Copy, Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CacheStatus {
    Hit,
    Miss,
}

/// Error payload returned by the HTTP API.
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[cfg(test)]
mod tests {
    use super::{DecisionReason, is_blocked_reason};

    #[test]
    fn maps_parental_reason() {
        assert!(matches!(
            DecisionReason::from_adguard_reason("FilteredParental"),
            DecisionReason::BlockedAdult
        ));
    }

    #[test]
    fn maps_unknown_filtered_to_blocked_rule() {
        assert!(is_blocked_reason("FilteredBlackList"));
    }

    #[test]
    fn maps_allowlist_reason() {
        assert!(matches!(
            DecisionReason::from_adguard_reason("NotFilteredAllowList"),
            DecisionReason::AllowedAllowlist
        ));
    }
}