//! The Minecraft: Java Edition catalog.

use crate::types::catalog::CatalogId;
use chrono::{DateTime, Utc};
use core::fmt::{self, Display, Formatter};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use utoipa::ToSchema;

/// Server distributions Warehouse tracks builds for.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, ToSchema,
)]
#[serde(rename_all = "lowercase")]
pub enum Distribution {
    /// Mojang's own server jar.
    ///
    /// Implicitly available for every release in the catalog and therefore never listed
    /// in [`MinecraftVersion::distributions`]. Consumers name it to model a user's
    /// choice, not to look it up.
    Vanilla,
    /// PaperMC's Paper.
    Paper,
    /// PurpurMC's Purpur.
    Purpur,
    /// FabricMC's loader.
    Fabric,
    /// QuiltMC's loader.
    Quilt,
    /// MinecraftForge's loader.
    Forge,
    /// NeoForged's loader.
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

/// The builds or loader versions available for one distribution of one Minecraft version.
///
/// Upstreams publish these in two shapes: dense integer build numbers, which compress well
/// as a range with holes, and opaque version strings, which do not.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ToSchema)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum BuildSet {
    /// A contiguous integer range minus the builds that are missing from it.
    Range {
        /// Prepended to each build number to form the full version string, when the
        /// upstream numbers its builds within a versioned line (NeoForge does this).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        prefix: Option<String>,
        /// Lowest build number, inclusive.
        min: u16,
        /// Highest build number, inclusive.
        max: u16,
        /// Build numbers inside the range that do not exist.
        #[serde(default, skip_serializing_if = "BTreeSet::is_empty")]
        excluded: BTreeSet<u16>,
    },
    /// An explicit list of version strings, newest first.
    Set {
        /// The available versions.
        values: Vec<String>,
    },
}

impl BuildSet {
    /// Whether the set contains no usable build at all.
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

/// One Minecraft release and everything known about running a server for it.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinecraftVersion {
    /// The Mojang version identifier, for example `1.21.4`.
    pub id: String,
    /// The Java major version this release is expected to run on.
    pub recommended_java: u8,
    /// Whether this release supports data packs.
    pub data_pack: bool,
    /// Available builds per distribution. A distribution missing from the map has no
    /// build for this Minecraft version.
    ///
    /// [`Distribution::Vanilla`] never appears here: Mojang's own server jar exists for
    /// every listed release and has no build numbers of its own, so its presence carries
    /// no information.
    pub distributions: BTreeMap<Distribution, BuildSet>,
}

/// The `minecraft` catalog document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MinecraftCatalog {
    /// Schema version of this document.
    pub schema: u32,
    /// When this document was built from upstream.
    pub updated_at: DateTime<Utc>,
    /// Releases, newest first. Only full releases appear; snapshots are excluded.
    pub versions: Vec<MinecraftVersion>,
}

impl MinecraftCatalog {
    /// Which catalog this document belongs to.
    pub const ID: CatalogId = CatalogId::Minecraft;

    /// Looks up a release by its Mojang identifier.
    #[must_use]
    pub fn version(&self, id: &str) -> Option<&MinecraftVersion> {
        self.versions.iter().find(|version| version.id == id)
    }

    /// The newest release, which is what a consumer should default to.
    #[must_use]
    pub fn latest(&self) -> Option<&MinecraftVersion> {
        self.versions.first()
    }
}
