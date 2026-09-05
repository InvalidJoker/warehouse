use crate::error::ResolveError;
use crate::http;
use crate::minecraft::build_range;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use warehouse_common::BuildSet;

const UPSTREAM: &str = "purpurmc";

#[derive(Debug, Deserialize)]
struct Project {
    versions: HashSet<String>,
}

#[derive(Debug, Deserialize)]
struct Version {
    builds: Builds,
}

#[derive(Debug, Deserialize)]
struct Builds {
    all: Vec<String>,
}

pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
    project: &str,
    known: &HashSet<String>,
) -> Result<HashMap<String, BuildSet>, ResolveError> {
    let listing: Project =
        http::json(client.get(format!("{base_url}v2/{project}")), UPSTREAM).await?;

    let mut resolved = HashMap::new();
    for version in listing.versions {
        if !known.contains(&version) {
            continue;
        }

        let detail: Version = http::json(
            client.get(format!("{base_url}v2/{project}/{version}")),
            UPSTREAM,
        )
        .await?;

        let builds: HashSet<u16> = detail
            .builds
            .all
            .iter()
            .filter_map(|build| build.parse().ok())
            .collect();

        if let Some(set) = build_range(None, &builds) {
            resolved.insert(version, set);
        }
    }

    Ok(resolved)
}
