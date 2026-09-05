pub mod types;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "sdk")]
pub mod sdk;

pub use types::catalog::{ALL_CATALOGS, CatalogEntry, CatalogId, Manifest, UnknownCatalogId};
pub use types::document::{Document, DocumentMismatch};
pub use types::java::{JavaCatalog, JavaMajor, JavaVersion};
pub use types::minecraft::{BuildSet, Distribution, MinecraftCatalog, MinecraftVersion};
pub use types::proxy::{ProxyCatalog, VelocityVersion};
pub use types::runtime::RuntimeCatalog;
pub use types::status::{CatalogStatus, StatusReport};
pub use types::version::Version;

pub const SCHEMA_VERSION: u32 = 1;
