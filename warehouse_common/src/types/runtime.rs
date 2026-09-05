//! Catalogs for language runtimes published as Docker Hub official images.

use crate::types::catalog::CatalogId;
use crate::types::version::Version;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The `go`, `node`, `python` and `rust` catalog document.
///
/// All four have the same shape: a flat list of `major.minor.patch` releases. Java is
/// the exception and has [`crate::JavaCatalog`] instead, because Temurin's
/// versioning does not fit a triple.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RuntimeCatalog {
    /// Schema version of this document.
    pub schema: u32,
    /// Which runtime this document describes.
    pub runtime: CatalogId,
    /// When this document was built from upstream.
    pub updated_at: DateTime<Utc>,
    /// Available releases, newest first.
    pub versions: Vec<Version>,
}

impl RuntimeCatalog {
    /// The newest release, which is what a consumer should default to.
    #[must_use]
    pub fn latest(&self) -> Option<Version> {
        self.versions.first().copied()
    }
}
