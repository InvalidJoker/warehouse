use crate::error::ResolveError;
use crate::http;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use warehouse_common::BuildSet;

#[derive(Debug, Deserialize)]
struct GameVersion {
    version: String,
}

#[derive(Debug, Deserialize)]
struct LoaderEntry {
    loader: LoaderVersion,
}

#[derive(Debug, Deserialize)]
struct LoaderVersion {
    version: String,
}

pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
    upstream: &'static str,
    known: &HashSet<String>,
) -> Result<HashMap<String, BuildSet>, ResolveError> {
    let supported: Vec<GameVersion> =
        http::json(client.get(format!("{base_url}/versions/game")), upstream).await?;

    let mut resolved = HashMap::new();
    for game in supported {
        if !known.contains(&game.version) {
            continue;
        }

        let loaders: Result<Vec<LoaderEntry>, _> = http::json(
            client.get(format!("{base_url}/versions/loader/{}", game.version)),
            upstream,
        )
        .await;

        match loaders {
            Ok(loaders) if !loaders.is_empty() => {
                resolved.insert(
                    game.version,
                    BuildSet::Set {
                        values: loaders
                            .into_iter()
                            .map(|entry| entry.loader.version)
                            .collect(),
                    },
                );
            }
            Ok(_) => {}
            Err(err) => {
                tracing::warn!(
                    upstream,
                    version = %game.version,
                    error = %err,
                    "skipping loader versions for this release"
                );
            }
        }
    }

    Ok(resolved)
}
