//! In-memory TTL cache used to avoid repeated AdGuard lookups for hot domains.

use std::{collections::HashMap, sync::Arc, time::Instant};

use tokio::sync::RwLock;

use crate::models::DomainCheckResponse;

/// Simple concurrent cache keyed by normalized domain name.
#[derive(Clone, Default)]
pub struct DecisionCache {
    inner: Arc<RwLock<HashMap<String, CacheEntry>>>,
}

impl DecisionCache {
    /// Returns a cached response when it is still fresh, removing expired entries lazily.
    pub async fn get_fresh(&self, domain: &str) -> Option<DomainCheckResponse> {
        {
            let cache = self.inner.read().await;
            if let Some(entry) = cache.get(domain) {
                if entry.expires_at > Instant::now() {
                    let mut response = entry.response.clone();
                    response.ttl = entry.remaining_ttl_secs();

                    return Some(response);
                }
            }
        }

        let mut cache = self.inner.write().await;
        if let Some(entry) = cache.get(domain) {
            if entry.expires_at <= Instant::now() {
                cache.remove(domain);
            }
        }

        None
    }

    /// Stores a new response using the response TTL as the cache expiry.
    pub async fn insert(&self, response: DomainCheckResponse) {
        let expires_at = Instant::now() + std::time::Duration::from_secs(response.ttl);
        let domain = response.domain.clone();
        let entry = CacheEntry {
            expires_at,
            response,
        };

        let mut cache = self.inner.write().await;
        cache.insert(domain, entry);
    }
}

/// Internal cache record pairing a response with its expiry timestamp.
struct CacheEntry {
    expires_at: Instant,
    response: DomainCheckResponse,
}

impl CacheEntry {
    /// Returns the remaining TTL exposed back to the caller.
    fn remaining_ttl_secs(&self) -> u64 {
        self.expires_at
            .saturating_duration_since(Instant::now())
            .as_secs()
            .max(1)
    }
}