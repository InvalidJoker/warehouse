//! Catalog identifiers and the manifest listing what an instance serves.

use chrono::{DateTime, Utc};
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// The catalogs a Warehouse instance can serve.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogId {
    /// Minecraft: Java Edition server versions and their server distributions.
    Minecraft,
    /// Minecraft proxy software (Velocity, BungeeCord).
    MinecraftProxy,
    /// Go toolchain releases published as `library/golang` tags.
    Go,
    /// Eclipse Temurin JDK releases published as `library/eclipse-temurin` tags.
    Java,
    /// Node.js releases published as `library/node` tags.
    Node,
    /// CPython releases published as `library/python` tags.
    Python,
    /// Rust toolchain releases published as `library/rust` tags.
    Rust,
}

/// Every catalog, in a stable order.
pub const ALL_CATALOGS: [CatalogId; 7] = [
    CatalogId::Minecraft,
    CatalogId::MinecraftProxy,
    CatalogId::Go,
    CatalogId::Java,
    CatalogId::Node,
    CatalogId::Python,
    CatalogId::Rust,
];

impl CatalogId {
    /// The identifier as it appears in URLs and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Minecraft => "minecraft",
            Self::MinecraftProxy => "minecraft-proxy",
            Self::Go => "go",
            Self::Java => "java",
            Self::Node => "node",
            Self::Python => "python",
            Self::Rust => "rust",
        }
    }

    /// Whether this catalog is sourced from Docker Hub's tag listing API.
    ///
    /// Docker Hub applies aggressive anonymous rate limits, so a server refreshes these
    /// catalogs behind a shared gate rather than independently.
    #[must_use]
    pub const fn uses_docker_hub(self) -> bool {
        matches!(
            self,
            Self::Go | Self::Java | Self::Node | Self::Python | Self::Rust
        )
    }
}

impl Display for CatalogId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Returned when a string does not name a known catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCatalogId(pub String);

impl Display for UnknownCatalogId {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        write!(formatter, "unknown catalog id {:?}", self.0)
    }
}

impl core::error::Error for UnknownCatalogId {}

impl FromStr for CatalogId {
    type Err = UnknownCatalogId;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        ALL_CATALOGS
            .into_iter()
            .find(|catalog| catalog.as_str() == value)
            .ok_or_else(|| UnknownCatalogId(value.to_owned()))
    }
}

/// What an instance currently holds, returned by `GET /catalog`.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Manifest {
    /// Schema version of the documents this instance serves.
    pub schema: u32,
    /// One entry per catalog the instance has data for.
    pub catalogs: Vec<CatalogEntry>,
}

/// A single catalog's freshness, as advertised by the manifest.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogEntry {
    /// Which catalog this describes.
    pub id: CatalogId,
    /// Opaque token identifying the current document.
    ///
    /// Send it back as `If-None-Match` to skip transferring an unchanged catalog.
    pub etag: String,
    /// When the held document was last successfully rebuilt from upstream.
    pub updated_at: DateTime<Utc>,
    /// Whether the last refresh attempt failed.
    ///
    /// A stale catalog is still served — the previous document remains valid and is
    /// almost always more useful than an error — but a consumer may want to warn.
    pub stale: bool,
}
