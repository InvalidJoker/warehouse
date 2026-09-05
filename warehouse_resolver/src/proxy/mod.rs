//! Builds the `minecraft-proxy` catalog.

mod bungeecord;
mod velocity;

use crate::error::ResolveError;
use chrono::Utc;
use warehouse_common::{ProxyCatalog, SCHEMA_VERSION};

const VELOCITY_URL: &str = "https://fill.papermc.io/";
const BUNGEECORD_URL: &str = "https://hub.spigotmc.org/jenkins/";

/// Rebuilds the `minecraft-proxy` catalog.
///
/// Unlike the Minecraft catalog, both halves are required: there are only two proxies,
/// and a document holding just one of them would silently remove the other from every
/// consumer's choices.
///
/// # Errors
///
/// Fails if either upstream is unreachable or returns nothing usable.
pub async fn resolve(client: &reqwest::Client) -> Result<ProxyCatalog, ResolveError> {
    Ok(ProxyCatalog {
        schema: SCHEMA_VERSION,
        updated_at: Utc::now(),
        velocity: velocity::resolve(client, VELOCITY_URL).await?,
        bungeecord: bungeecord::resolve(client, BUNGEECORD_URL).await?,
    })
}
