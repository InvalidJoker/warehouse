use super::*;
use crate::types::status::StatusReport;
use zelus::service;

#[service(path = "/system", tag = "system")]
pub trait SystemService {
    /// Reports that the instance is up. Says nothing about catalog freshness.
    #[route("/health", method = GET, no_auth)]
    #[error()]
    async fn health(&self) -> Result<(), _>;

    /// Reports each catalog's freshness, last error and next scheduled refresh.
    #[route("/status", method = GET)]
    #[error()]
    async fn status(&self) -> Result<StatusReport, _>;
}
