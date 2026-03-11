//! HTTP entrypoint for the relay service.
//!
//! This module wires together configuration loading, tracing setup, shared
//! application state, and the two public HTTP routes exposed by the service.

use anyhow::Result;
mod adguard;
mod cache;
mod config;
mod domain;
mod models;

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderMap, StatusCode, header::AUTHORIZATION},
    response::{IntoResponse, Response},
    routing::get,
};
use cache::DecisionCache;
use chrono::Utc;
use config::Settings;
use logroller::{Compression, LogRollerBuilder, Rotation, RotationAge, TimeZone};
use models::{CacheStatus, DomainCheckQuery, DomainCheckResponse, ErrorResponse};
use tokio::net::TcpListener;
use tracing_subscriber::util::SubscriberInitExt;

/// Shared state injected into every request handler.
#[derive(Clone)]
struct AppState {
    settings: Arc<Settings>,
    adguard: adguard::AdguardClient,
    cache: DecisionCache,
}

/// Starts the relay HTTP server and installs file-based tracing.
#[tokio::main]
async fn main() -> Result<()> {
    // Keep operational logs on disk so the container can run without relying on stdout scraping.
    let appender = LogRollerBuilder::new("./logs", "tracing.log")
        .rotation(Rotation::AgeBased(RotationAge::Daily))
        .max_keep_files(30)
        .time_zone(TimeZone::Local)
        .compression(Compression::Gzip)
        .build()?;

    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);

    tracing_subscriber::fmt()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(false)
        .with_file(true)
        .with_line_number(true)
        .finish()
        .try_init()?;

    let settings = Arc::new(Settings::from_env()?);

    let state = AppState {
        adguard: adguard::AdguardClient::new(settings.adguard_base_url.clone()),
        cache: DecisionCache::default(),
        settings: settings.clone(),
    };

    let app = Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/domain-check", get(get_domain_check))
        .with_state(state);

    let listener = TcpListener::bind(settings.bind_addr).await?;

    tracing::info!(listen_addr = %settings.bind_addr, "relay listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

/// Lightweight readiness endpoint for container and reverse-proxy health checks.
async fn healthz() -> impl IntoResponse {
    "OK"
}

/// Authenticated route handler for domain lookup requests.
async fn get_domain_check(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<DomainCheckQuery>,
) -> Result<Json<DomainCheckResponse>, ApiError> {
    authorize(&headers, &state.settings.auth_token)?;
    resolve_domain_check(state, query.domain).await.map(Json)
}

/// Resolves a hostname against the local cache first and AdGuard second.
async fn resolve_domain_check(
    state: AppState,
    raw_domain: String,
) -> Result<DomainCheckResponse, ApiError> {
    let domain = domain::normalize_domain(&raw_domain)
        .map_err(|err| ApiError::BadRequest(err.to_string()))?;

    if let Some(cached) = state.cache.get_fresh(&domain).await {
        return Ok(cached.with_cache_status(CacheStatus::Hit));
    }

    let upstream = state
        .adguard
        .check_host(&domain)
        .await
        .map_err(|err| {
            tracing::error!(error = %err, domain, "adguard lookup failed");
            ApiError::Upstream
        })?;

    let blocked = models::is_blocked_reason(&upstream.reason);
    let ttl = if blocked {
        state.settings.blocked_ttl_secs
    } else {
        state.settings.allowed_ttl_secs
    };

    let response =
        DomainCheckResponse::from_adguard(domain, upstream, ttl, CacheStatus::Miss, Utc::now());

    state.cache.insert(response.clone()).await;

    Ok(response)
}

/// Validates the bearer token supplied by the caller.
fn authorize(headers: &HeaderMap, expected_token: &str) -> Result<(), ApiError> {
    let value = headers
        .get(AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(ApiError::Unauthorized)?;

    let provided = value
        .strip_prefix("Bearer ")
        .filter(|token| !token.is_empty())
        .ok_or(ApiError::Unauthorized)?;

    if provided == expected_token {
        Ok(())
    } else {
        Err(ApiError::Unauthorized)
    }
}

/// Waits for Ctrl+C so the HTTP server can shut down gracefully.
async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::error!(error = %err, "failed to install shutdown signal handler");
    }
}

/// Minimal HTTP error surface returned to clients.
enum ApiError {
    Unauthorized,
    BadRequest(String),
    Upstream,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            Self::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                Json(ErrorResponse {
                    error: "unauthorized".to_owned(),
                }),
            )
                .into_response(),
            Self::BadRequest(message) => (
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse { error: message }),
            )
                .into_response(),
            Self::Upstream => (
                StatusCode::BAD_GATEWAY,
                Json(ErrorResponse {
                    error: "upstream service unavailable".to_owned(),
                }),
            )
                .into_response(),
        }
    }
}
