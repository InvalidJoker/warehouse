use crate::error::ResolveError;
use crate::http;
use crate::minecraft::build_range;
use serde::Deserialize;
use std::collections::HashSet;
use warehouse_common::VelocityVersion;

const UPSTREAM: &str = "papermc";
const PROJECT: &str = "velocity";

const DEFAULT_JAVA: u8 = 21;

#[derive(Debug, Deserialize)]
struct VersionsResponse {
    versions: Vec<VersionEntry>,
}

#[derive(Debug, Deserialize)]
struct VersionEntry {
    version: Version,
    builds: Vec<u16>,
}

#[derive(Debug, Deserialize)]
struct Version {
    id: String,
    #[serde(default)]
    java: Option<Java>,
}

#[derive(Debug, Deserialize)]
struct Java {
    version: JavaVersion,
}

#[derive(Debug, Deserialize)]
struct JavaVersion {
    minimum: u8,
}

pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<VelocityVersion>, ResolveError> {
    let response: VersionsResponse = http::json(
        client.get(format!("{base_url}v3/projects/{PROJECT}/versions")),
        UPSTREAM,
    )
    .await?;

    let versions: Vec<VelocityVersion> = response
        .versions
        .into_iter()
        .filter_map(|entry| {
            let builds: HashSet<u16> = entry.builds.into_iter().collect();
            Some(VelocityVersion {
                id: entry.version.id,
                java: entry
                    .version
                    .java
                    .map_or(DEFAULT_JAVA, |java| java.version.minimum),
                builds: build_range(None, &builds)?,
            })
        })
        .collect();

    if versions.is_empty() {
        return Err(ResolveError::Empty { upstream: UPSTREAM });
    }

    Ok(versions)
}
