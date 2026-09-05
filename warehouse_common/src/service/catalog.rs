use super::*;
use crate::types::catalog::{CatalogId, Manifest};
use crate::types::java::JavaCatalog;
use crate::types::minecraft::MinecraftCatalog;
use crate::types::proxy::ProxyCatalog;
use crate::types::runtime::RuntimeCatalog;
use zelus::service;

#[service(path = "/catalog", tag = "catalog")]
pub trait CatalogService {
    /// Lists every catalog this instance holds, with its schema, etag and freshness.
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

    /// Queues a catalog rebuild instead of waiting for its next scheduled refresh.
    #[route("/{catalog}/refresh", method = POST)]
    #[error(catalog::not_found)]
    async fn refresh_catalog(&self, catalog: CatalogId) -> Result<(), _>;
}
