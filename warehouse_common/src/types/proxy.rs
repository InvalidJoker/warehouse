use crate::types::catalog::CatalogId;
use crate::types::minecraft::BuildSet;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VelocityVersion {
    pub id: String,
    pub java: u8,
    pub builds: BuildSet,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ProxyCatalog {
    pub schema: u32,
    pub updated_at: DateTime<Utc>,
    pub velocity: Vec<VelocityVersion>,
    pub bungeecord: Vec<u16>,
}

impl ProxyCatalog {
    pub const ID: CatalogId = CatalogId::MinecraftProxy;
}
