//! A catalog document of any kind.

use crate::types::catalog::CatalogId;
use crate::types::java::JavaCatalog;
use crate::types::minecraft::MinecraftCatalog;
use crate::types::proxy::ProxyCatalog;
use crate::types::runtime::RuntimeCatalog;
use chrono::{DateTime, Utc};
use serde::Serialize;

/// A finished catalog document.
///
/// Catalogs do not share a shape — a Minecraft release and a Go release have nothing in
/// common — so anything that handles them generically holds this instead.
#[derive(Debug, Clone, Serialize)]
#[serde(untagged)]
pub enum Document {
    /// The `minecraft` catalog.
    Minecraft(Box<MinecraftCatalog>),
    /// The `minecraft-proxy` catalog.
    Proxy(Box<ProxyCatalog>),
    /// The `java` catalog.
    Java(Box<JavaCatalog>),
    /// One of the `major.minor.patch` runtime catalogs.
    Runtime(Box<RuntimeCatalog>),
}

/// A stored document did not match the catalog it was stored under.
#[derive(Debug, thiserror::Error)]
#[error("stored document does not match catalog {catalog}: {source}")]
pub struct DocumentMismatch {
    /// The catalog the document was stored under.
    pub catalog: CatalogId,
    /// Why it did not decode.
    #[source]
    pub source: serde_json::Error,
}

impl Document {
    /// Decodes a document knowing which catalog it belongs to.
    ///
    /// The variants are structurally distinct but not self-describing, so the catalog id
    /// is what picks the shape rather than a tag inside the document.
    ///
    /// # Errors
    ///
    /// Fails if the value is not that catalog's document.
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

    /// Which catalog this document belongs to.
    #[must_use]
    pub const fn catalog(&self) -> CatalogId {
        match self {
            Self::Minecraft(_) => CatalogId::Minecraft,
            Self::Proxy(_) => CatalogId::MinecraftProxy,
            Self::Java(_) => CatalogId::Java,
            Self::Runtime(runtime) => runtime.runtime,
        }
    }

    /// When the document was built from upstream.
    #[must_use]
    pub const fn updated_at(&self) -> DateTime<Utc> {
        match self {
            Self::Minecraft(catalog) => catalog.updated_at,
            Self::Proxy(catalog) => catalog.updated_at,
            Self::Java(catalog) => catalog.updated_at,
            Self::Runtime(catalog) => catalog.updated_at,
        }
    }

    /// The Minecraft catalog, when this is one.
    #[must_use]
    pub const fn as_minecraft(&self) -> Option<&MinecraftCatalog> {
        match self {
            Self::Minecraft(catalog) => Some(catalog),
            _ => None,
        }
    }

    /// The proxy catalog, when this is one.
    #[must_use]
    pub const fn as_proxy(&self) -> Option<&ProxyCatalog> {
        match self {
            Self::Proxy(catalog) => Some(catalog),
            _ => None,
        }
    }

    /// The Java catalog, when this is one.
    #[must_use]
    pub const fn as_java(&self) -> Option<&JavaCatalog> {
        match self {
            Self::Java(catalog) => Some(catalog),
            _ => None,
        }
    }

    /// The runtime catalog, when this is one.
    #[must_use]
    pub const fn as_runtime(&self) -> Option<&RuntimeCatalog> {
        match self {
            Self::Runtime(catalog) => Some(catalog),
            _ => None,
        }
    }
}
