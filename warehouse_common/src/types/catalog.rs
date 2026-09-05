use chrono::{DateTime, Utc};
use core::fmt::{self, Display, Formatter};
use core::str::FromStr;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "kebab-case")]
pub enum CatalogId {
    Minecraft,
    MinecraftProxy,
    Go,
    Java,
    Node,
    Python,
    Rust,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Manifest {
    pub schema: u32,
    pub catalogs: Vec<CatalogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CatalogEntry {
    pub id: CatalogId,
    pub etag: String,
    pub updated_at: DateTime<Utc>,
    pub stale: bool,
}
