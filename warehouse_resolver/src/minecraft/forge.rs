use crate::error::ResolveError;
use crate::maven;
use std::collections::{HashMap, HashSet};
use warehouse_common::BuildSet;

const UPSTREAM: &str = "minecraftforge";

pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
    known: &HashSet<String>,
) -> Result<HashMap<String, BuildSet>, ResolveError> {
    let versions = maven::versions(
        client,
        &format!("{base_url}/net/minecraftforge/forge/maven-metadata.xml"),
        UPSTREAM,
    )
    .await?;

    let mut grouped: HashMap<String, Vec<String>> = HashMap::new();
    for version in versions {
        let Some((minecraft, loader)) = version.split_once('-') else {
            continue;
        };
        if !known.contains(minecraft) {
            continue;
        }
        grouped
            .entry(minecraft.to_owned())
            .or_default()
            .push(loader.to_owned());
    }

    Ok(grouped
        .into_iter()
        .map(|(minecraft, mut loaders)| {
            loaders.reverse();
            (minecraft, BuildSet::Set { values: loaders })
        })
        .collect())
}
