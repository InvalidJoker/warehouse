//! Reading and refreshing catalogs.

#[allow(
    clippy::wildcard_imports,
    reason = "the service macro resolves error and path-variable modules from this scope"
)]
use super::*;
use crate::types::catalog::{CatalogId, Manifest};
use crate::types::java::JavaCatalog;
use crate::types::minecraft::MinecraftCatalog;
use crate::types::proxy::ProxyCatalog;
use crate::types::runtime::RuntimeCatalog;
use zelus::service;

#[service(path = "/catalog", tag = "catalog")]
pub trait CatalogService {
    /// Lists every catalog this instance holds, with its etag and freshness.
    ///
    /// A consumer polls this and refetches only the catalogs whose etag changed. It also
    /// carries the instance's schema version, which a consumer should compare against its
    /// own before trusting any document.
    #[route("", method = GET)]
    #[error()]
    async fn manifest(&self) -> Result<Manifest, _>;

    /// Returns the Minecraft: Java Edition catalog.
    #[route("/minecraft", method = GET)]
    #[error(catalog::unavailable)]
    async fn minecraft_catalog(&self) -> Result<MinecraftCatalog, _>;

    /// Returns the Minecraft proxy catalog.
    #[route("/minecraft-proxy", method = GET)]
    #[error(catalog::unavailable)]
    async fn proxy_catalog(&self) -> Result<ProxyCatalog, _>;

    /// Returns the Eclipse Temurin JDK catalog.
    #[route("/java", method = GET)]
    #[error(catalog::unavailable)]
    async fn java_catalog(&self) -> Result<JavaCatalog, _>;

    /// Returns one of the `major.minor.patch` language runtime catalogs.
    #[route("/runtime/{runtime}", method = GET)]
    #[error(catalog::{not_found, unavailable})]
    async fn runtime_catalog(&self, runtime: CatalogId) -> Result<RuntimeCatalog, _>;

    /// Rebuilds a catalog now instead of waiting for its next scheduled refresh.
    ///
    /// Returns as soon as the refresh is queued; `GET /system/status` reports how it went.
    #[route("/{catalog}/refresh", method = POST)]
    #[error(catalog::not_found)]
    async fn refresh_catalog(&self, catalog: CatalogId) -> Result<(), _>;
}
