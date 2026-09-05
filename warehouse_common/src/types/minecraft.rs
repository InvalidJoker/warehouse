use crate::types::catalog::CatalogId;
use chrono::{DateTime, Utc};
use core::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::ToSchema;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    Vanilla,
    Paper,
    Purpur,
    Fabric,
    Quilt,
    Forge,
    Neoforge,
}

impl Display for Distribution {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Vanilla => "vanilla",
            Self::Paper => "paper",
            Self::Purpur => "purpur",
            Self::Fabric => "fabric",
            Self::Quilt => "quilt",
            Self::Forge => "forge",
            Self::Neoforge => "neoforge",
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BuildSet {
    Range {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        min: u16,
        max: u16,
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        excluded: BTreeSet<u16>,
    },
    Set {
        values: Vec<String>,
    },
}

impl BuildSet {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        match self {
            Self::Range {
                min, max, excluded, ..
            } => (*min..=*max).all(|build| excluded.contains(&build)),
            Self::Set { values } => values.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinecraftVersion {
    pub id: String,
    pub recommended_java: u8,
    pub data_pack: bool,
    pub distributions: BTreeMap<Distribution, BuildSet>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinecraftCatalog {
    pub schema: u32,
    pub updated_at: DateTime<Utc>,
    pub versions: Vec<MinecraftVersion>,
}

impl MinecraftCatalog {
    pub const ID: CatalogId = CatalogId::Minecraft;

    #[must_use]
    pub fn version(&self, id: &str) -> Option<&MinecraftVersion> {
        self.versions.iter().find(|version| version.id == id)
    }

    #[must_use]
    pub fn latest(&self) -> Option<&MinecraftVersion> {
        self.versions.first()
    }
}
