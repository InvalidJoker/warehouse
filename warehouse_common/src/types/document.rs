use crate::types::catalog::CatalogId;
use crate::types::java::JavaCatalog;
use crate::types::minecraft::MinecraftCatalog;
use crate::types::proxy::ProxyCatalog;
use crate::types::runtime::RuntimeCatalog;
use chrono::{DateTime, Utc};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Document {
    Minecraft(Box<MinecraftCatalog>),
    Proxy(Box<ProxyCatalog>),
    Java(Box<JavaCatalog>),
    Runtime(Box<RuntimeCatalog>),
}

#[derive(Debug, thiserror::Error)]
#[error("stored document does not match catalog {catalog}: {source}")]
pub struct DocumentMismatch {
    pub catalog: CatalogId,
    #[source]
    pub source: serde_json::Error,
}

impl Document {
    pub fn from_value(
        catalog: CatalogId,
        value: serde_json::Value,
    ) -> Result<Self, DocumentMismatch> {
        fn decode<T: serde::de::DeserializeOwned>(
            catalog: CatalogId,
            value: serde_json::Value,
        ) -> Result<Box<T>, DocumentMismatch> {
            serde_json::from_value(value)
                .map(Box::new)
                .map_err(|source| DocumentMismatch { catalog, source })
        }

        Ok(match catalog {
            CatalogId::Minecraft => Self::Minecraft(decode(catalog, value)?),
            CatalogId::MinecraftProxy => Self::Proxy(decode(catalog, value)?),
            CatalogId::Java => Self::Java(decode(catalog, value)?),
            CatalogId::Go | CatalogId::Node | CatalogId::Python | CatalogId::Rust => {
                Self::Runtime(decode(catalog, value)?)
            }
        })
    }

    #[must_use]
    pub const fn catalog(&self) -> CatalogId {
        match self {
            Self::Minecraft(_) => CatalogId::Minecraft,
            Self::Proxy(_) => CatalogId::MinecraftProxy,
            Self::Java(_) => CatalogId::Java,
            Self::Runtime(runtime) => runtime.runtime,
        }
    }

    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        match self {
            Self::Minecraft(catalog) => catalog.updated_at,
            Self::Proxy(catalog) => catalog.updated_at,
            Self::Java(catalog) => catalog.updated_at,
            Self::Runtime(catalog) => catalog.updated_at,
        }
    }

    #[must_use]
    pub const fn as_minecraft(&self) -> Option<&MinecraftCatalog> {
        match self {
            Self::Minecraft(catalog) => Some(catalog),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_proxy(&self) -> Option<&ProxyCatalog> {
        match self {
            Self::Proxy(catalog) => Some(catalog),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_java(&self) -> Option<&JavaCatalog> {
        match self {
            Self::Java(catalog) => Some(catalog),
            _ => None,
        }
    }

    #[must_use]
    pub const fn as_runtime(&self) -> Option<&RuntimeCatalog> {
        match self {
            Self::Runtime(catalog) => Some(catalog),
            _ => None,
        }
    }
}
