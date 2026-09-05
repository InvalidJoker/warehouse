use crate::error::ResolveError;
use crate::http;
use crate::minecraft::build_range;
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use warehouse_common::BuildSet;

const UPSTREAM: &str = "papermc";

#[derive(Debug, Deserialize)]
struct VersionsResponse {
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionEntry {
    version: VersionId,
}

#[derive(Debug, Deserialize)]
struct VersionId {
    id: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum Channel {
    Alpha,
    Beta,
    Stable,
    Recommended,
}

#[derive(Debug, Deserialize)]
struct Build {
    id: u16,
    channel: Channel,
}

pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
    project: &str,
    known: &HashSet<String>,
) -> Result<HashMap<String, BuildSet>, ResolveError> {
    let versions: VersionsResponse = http::json(
        client.get(format!("{base_url}v3/projects/{project}/versions")),
        UPSTREAM,
    )
    .await?;

    let mut resolved = HashMap::new();
    for entry in versions.versions {
        let version = entry.version.id;
        if !known.contains(&version) {
            continue;
        }

        let builds: Vec<Build> = http::json(
            client.get(format!(
                "{base_url}v3/projects/{project}/versions/{version}/builds"
            )),
            UPSTREAM,
        )
        .await?;

        let stable: HashSet<u16> = builds
            .into_iter()
            .filter(|build| matches!(build.channel, Channel::Stable | Channel::Recommended))
            .map(|build| build.id)
            .collect();

        if let Some(set) = build_range(None, &stable) {
            resolved.insert(version, set);
        }
    }

    Ok(resolved)
}
