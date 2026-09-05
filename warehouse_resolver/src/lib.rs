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
