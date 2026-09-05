//! Operational reporting, served by `GET /system/status`.

use crate::types::catalog::CatalogId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// How one catalog's recent refresh attempts went.
#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CatalogStatus {
    /// Which catalog this describes.
    pub id: Option<CatalogId>,
    /// Whether a document is currently being served.
    pub resolved: bool,
    /// Whether the last refresh attempt failed. The previous document is still served.
    pub stale: bool,
    /// When the catalog was last rebuilt successfully.
    pub last_success: Option<DateTime<Utc>>,
    /// When the last attempt failed.
    pub last_failure: Option<DateTime<Utc>>,
    /// The error from that failure, naming the upstream that caused it.
    pub last_error: Option<String>,
    /// Failures since the last success. Drives the retry backoff.
    pub consecutive_failures: u32,
    /// When the next refresh is currently scheduled for.
    pub next_attempt: Option<DateTime<Utc>>,
}

/// The instance's overall state.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatusReport {
    /// Schema version of the documents this instance serves.
    pub schema: u32,
    /// The running Warehouse version.
    pub version: String,
    /// One entry per configured catalog.
    pub catalogs: Vec<CatalogStatus>,
}
