use crate::types::catalog::CatalogId;
use crate::types::version::Version;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeCatalog {
    pub schema: u32,
    pub runtime: CatalogId,
    pub updated_at: DateTime<Utc>,
    pub versions: Vec<Version>,
}

impl RuntimeCatalog {
    #[must_use]
    pub fn latest(&self) -> Option<Version> {
        self.versions.first().copied()
    }
}
