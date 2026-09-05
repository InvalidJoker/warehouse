//! NeoForged's maven repository.

use crate::error::ResolveError;
use crate::maven;
use crate::minecraft::build_range;
use std::collections::{HashMap, HashSet};
use warehouse_common::BuildSet;

const UPSTREAM: &str = "neoforged";

/// Resolves loader builds per Minecraft version.
///
/// NeoForge encodes the Minecraft version in its own: loader `21.4.60` targets Minecraft
/// `1.21.4`. The recovered prefix travels with the build range so a consumer can rebuild
/// the full loader version from a build number alone.
pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
    known: &HashSet<String>,
) -> Result<HashMap<String, BuildSet>, ResolveError> {
    let versions = maven::versions(
        client,
        &format!("{base_url}/releases/net/neoforged/neoforge/maven-metadata.xml"),
        UPSTREAM,
    )
    .await?;

    let mut grouped: HashMap<(u16, u16), HashSet<u16>> = HashMap::new();
    for version in versions {
        let mut parts = version.splitn(3, '.');
        let (Some(minor), Some(patch), Some(build)) = (parts.next(), parts.next(), parts.next())
        else {
            continue;
        };
        let (Ok(minor), Ok(patch), Ok(build)) = (
            minor.parse::<u16>(),
            patch.parse::<u16>(),
            build.parse::<u16>(),
        ) else {
            continue;
        };
        grouped.entry((minor, patch)).or_default().insert(build);
    }

    let mut resolved = HashMap::new();
    for ((minor, patch), builds) in grouped {
        let minecraft = format!("1.{minor}.{patch}");
        if !known.contains(&minecraft) {
            continue;
        }
        if let Some(set) = build_range(Some(format!("{minor}.{patch}.")), &builds) {
            resolved.insert(minecraft, set);
        }
    }

    Ok(resolved)
}
