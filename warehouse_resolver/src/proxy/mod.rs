mod bungeecord;
mod velocity;

use crate::error::ResolveError;
use chrono::Utc;
use warehouse_common::{ProxyCatalog, SCHEMA_VERSION};

const VELOCITY_URL: &str = "https://fill.papermc.io/";
const BUNGEECORD_URL: &str = "https://hub.spigotmc.org/jenkins/";

pub async fn resolve(client: &reqwest::Client) -> Result<ProxyCatalog, ResolveError> {
    Ok(ProxyCatalog {
        schema: SCHEMA_VERSION,
        updated_at: Utc::now(),
        velocity: velocity::resolve(client, VELOCITY_URL).await?,
        bungeecord: bungeecord::resolve(client, BUNGEECORD_URL).await?,
    })
}
