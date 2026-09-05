mod fabric;
mod forge;
mod neoforge;
mod paper;
mod purpur;

use crate::error::ResolveError;
use crate::http;
use chrono::Utc;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use warehouse_common::{
    BuildSet, Distribution, MinecraftCatalog, MinecraftVersion, SCHEMA_VERSION,
};

const MOJANG_MANIFEST: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const PAPER_URL: &str = "https://fill.papermc.io/";
const PURPUR_URL: &str = "https://api.purpurmc.org/";
const FABRIC_URL: &str = "https://meta.fabricmc.net/v2";
const QUILT_URL: &str = "https://meta.quiltmc.org/v3";
const FORGE_URL: &str = "https://maven.minecraftforge.net";
const NEOFORGE_URL: &str = "https://maven.neoforged.net";

const OLDEST_TRACKED: &str = "1.7.10";

const NEWEST_JAVA: u8 = 25;

const JAVA_THRESHOLDS: [(&str, u8); 3] = [("1.21.11", 21), ("1.20.4", 17), ("1.16.5", 8)];

const DATA_PACK_FLOOR: &str = "1.12.2";

#[derive(Debug, Deserialize)]
struct Manifest {
    versions: Vec<ManifestEntry>,
}

#[derive(Debug, Deserialize)]
struct ManifestEntry {
    id: String,
    #[serde(rename = "type")]
    kind: String,
}

pub(crate) fn build_range(prefix: Option<String>, builds: &HashSet<u16>) -> Option<BuildSet> {
    let min = *builds.iter().min()?;
    let max = *builds.iter().max()?;
    let excluded: BTreeSet<u16> = (min..=max)
        .filter(|build| !builds.contains(build))
        .collect();

    Some(BuildSet::Range {
        prefix,
        min,
        max,
        excluded,
    })
}

pub async fn resolve(client: &reqwest::Client) -> Result<MinecraftCatalog, ResolveError> {
    let manifest: Manifest = http::json(client.get(MOJANG_MANIFEST), "mojang").await?;

    let mut versions = Vec::new();
    let mut recommended_java = NEWEST_JAVA;
    let mut data_pack = true;

    for entry in manifest.versions {
        if entry.kind != "release" {
            continue;
        }

        if let Some((_, java)) = JAVA_THRESHOLDS
            .iter()
            .find(|(release, _)| *release == entry.id)
        {
            recommended_java = *java;
        }
        if entry.id == DATA_PACK_FLOOR {
            data_pack = false;
        }

        let last = entry.id == OLDEST_TRACKED;
        versions.push(MinecraftVersion {
            id: entry.id,
            recommended_java,
            data_pack,
            distributions: BTreeMap::new(),
        });

        if last {
            break;
        }
    }

    if versions.is_empty() {
        return Err(ResolveError::Empty { upstream: "mojang" });
    }

    let known: HashSet<String> = versions.iter().map(|version| version.id.clone()).collect();
    let mut merged: HashMap<Distribution, HashMap<String, BuildSet>> = HashMap::new();

    merge(
        &mut merged,
        Distribution::Paper,
        paper::resolve(client, PAPER_URL, "paper", &known).await,
    );
    merge(
        &mut merged,
        Distribution::Purpur,
        purpur::resolve(client, PURPUR_URL, "purpur", &known).await,
    );
    merge(
        &mut merged,
        Distribution::Fabric,
        fabric::resolve(client, FABRIC_URL, "fabricmc", &known).await,
    );
    merge(
        &mut merged,
        Distribution::Quilt,
        fabric::resolve(client, QUILT_URL, "quiltmc", &known).await,
    );
    merge(
        &mut merged,
        Distribution::Forge,
        forge::resolve(client, FORGE_URL, &known).await,
    );
    merge(
        &mut merged,
        Distribution::Neoforge,
        neoforge::resolve(client, NEOFORGE_URL, &known).await,
    );

    for version in &mut versions {
        for (distribution, builds) in &mut merged {
            if let Some(set) = builds.remove(&version.id) {
                version.distributions.insert(*distribution, set);
            }
        }
    }

    Ok(MinecraftCatalog {
        schema: SCHEMA_VERSION,
        updated_at: Utc::now(),
        versions,
    })
}

fn merge(
    target: &mut HashMap<Distribution, HashMap<String, BuildSet>>,
    distribution: Distribution,
    resolved: Result<HashMap<String, BuildSet>, ResolveError>,
) {
    match resolved {
        Ok(builds) => {
            tracing::debug!(%distribution, versions = builds.len(), "resolved distribution");
            target.insert(distribution, builds);
        }
        Err(err) => {
            tracing::warn!(%distribution, error = %err, "distribution unavailable, omitting it");
        }
    }
}
