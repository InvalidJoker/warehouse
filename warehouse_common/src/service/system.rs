//! Liveness and operational reporting.

#[allow(
    clippy::wildcard_imports,
    reason = "the service macro resolves error and path-variable modules from this scope"
)]
use super::*;
use crate::types::status::StatusReport;
use zelus::service;

#[service(path = "/system", tag = "system")]
pub trait SystemService {
    /// Reports that the instance is up.
    ///
    /// Unauthenticated, and deliberately says nothing about the catalogs: an instance
    /// that has not resolved anything yet is still healthy, and a probe that failed until
    /// every upstream answered would make a cold start look like an outage.
    #[route("/health", method = GET, no_auth)]
    #[error()]
    async fn health(&self) -> Result<(), _>;

    /// Reports each catalog's freshness, last error and next scheduled refresh.
    #[route("/status", method = GET)]
    #[error()]
    async fn status(&self) -> Result<StatusReport, _>;
}
