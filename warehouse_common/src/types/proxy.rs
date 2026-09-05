//! The Minecraft proxy catalog.

use crate::types::catalog::CatalogId;
use crate::types::minecraft::BuildSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

/// One Velocity release line and its builds.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VelocityVersion {
    /// The Velocity version identifier, for example `3.4.0-SNAPSHOT`.
    pub id: String,
    /// Minimum Java major version this line requires.
    pub java: u8,
    /// Builds published for this line.
    pub builds: BuildSet,
}

/// The `minecraft-proxy` catalog document.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProxyCatalog {
    /// Schema version of this document.
    pub schema: u32,
    /// When this document was built from upstream.
    pub updated_at: DateTime<Utc>,
    /// Velocity release lines, newest first.
    pub velocity: Vec<VelocityVersion>,
    /// Successful BungeeCord build numbers, newest first.
    pub bungeecord: Vec<u16>,
}

impl ProxyCatalog {
    /// Which catalog this document belongs to.
    pub const ID: CatalogId = CatalogId::MinecraftProxy;
}
