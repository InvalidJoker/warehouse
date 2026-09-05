//! Rebuilds Warehouse catalogs from their upstream sources.
//!
//! Each catalog has one entry point that performs every request it needs and returns a
//! finished document. Resolvers hold no state beyond the shared HTTP client and the
//! Docker Hub gate, so a caller decides entirely on its own when to refresh what.
//!
//! # The identifier invariant
//!
//! A resolver may only emit version identifiers. It must never place a download URL, an
//! image reference or a checksum into a catalog — see the crate documentation of
//! `warehouse_common` for why. Upstream URLs are constants inside this crate and stay here.

pub mod docker;
pub mod error;
mod http;
mod maven;
pub mod minecraft;
pub mod proxy;
pub mod runtime;

pub use docker::DockerGate;
pub use error::ResolveError;
pub use http::client;

use warehouse_common::CatalogId;
use warehouse_common::types::document::Document;

/// Rebuilds any catalog from upstream.
///
/// # Errors
///
/// Fails when the catalog's required upstreams cannot be read. Which upstreams are
/// required varies: see the per-catalog resolvers.
pub async fn resolve(
    client: &reqwest::Client,
    gate: &DockerGate,
    id: CatalogId,
) -> Result<Document, ResolveError> {
    Ok(match id {
        CatalogId::Minecraft => Document::Minecraft(Box::new(minecraft::resolve(client).await?)),
        CatalogId::MinecraftProxy => Document::Proxy(Box::new(proxy::resolve(client).await?)),
        CatalogId::Java => Document::Java(Box::new(runtime::resolve_java(client, gate).await?)),
        CatalogId::Go | CatalogId::Node | CatalogId::Python | CatalogId::Rust => {
            Document::Runtime(Box::new(runtime::resolve(client, gate, id).await?))
        }
    })
}
