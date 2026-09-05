use crate::types::catalog::CatalogId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub struct CatalogStatus {
    pub id: Option<CatalogId>,
    pub resolved: bool,
    pub stale: bool,
    pub last_success: Option<DateTime<Utc>>,
    pub last_failure: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub consecutive_failures: u32,
    pub next_attempt: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct StatusReport {
    pub schema: u32,
    pub version: String,
    pub catalogs: Vec<CatalogStatus>,
}
