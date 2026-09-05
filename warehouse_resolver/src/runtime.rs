use crate::docker::DockerGate;
use crate::error::ResolveError;
use chrono::Utc;
use warehouse_common::{
    CatalogId, JavaCatalog, JavaMajor, JavaVersion, RuntimeCatalog, SCHEMA_VERSION, Version,
};

const UPSTREAM: &str = "dockerhub";

const NODE_MINIMUM_MAJOR: u16 = 12;

const fn image(id: CatalogId) -> Option<&'static str> {
    match id {
        CatalogId::Go => Some("golang"),
        CatalogId::Java => Some("eclipse-temurin"),
        CatalogId::Node => Some("node"),
        CatalogId::Python => Some("python"),
        CatalogId::Rust => Some("rust"),
        CatalogId::Minecraft | CatalogId::MinecraftProxy => None,
    }
}

pub async fn resolve(
    client: &reqwest::Client,
    gate: &DockerGate,
    id: CatalogId,
) -> Result<RuntimeCatalog, ResolveError> {
    let image = image(id).ok_or(ResolveError::Empty { upstream: UPSTREAM })?;
    let tags = gate.acquire().await.list_tags(client, image).await?;

    let mut versions: Vec<Version> = tags
        .iter()
        .filter_map(|tag| tag.parse::<Version>().ok())
        .filter(|version| accepted(id, *version))
        .collect();

    versions.sort_unstable_by(|left, right| right.cmp(left));
    versions.dedup();

    if versions.is_empty() {
        return Err(ResolveError::Empty { upstream: UPSTREAM });
    }

    Ok(RuntimeCatalog {
        schema: SCHEMA_VERSION,
        runtime: id,
        updated_at: Utc::now(),
        versions,
    })
}

const fn accepted(id: CatalogId, version: Version) -> bool {
    match id {
        CatalogId::Node => version.major >= NODE_MINIMUM_MAJOR,
        CatalogId::Python => version.major > 3 || (version.major == 3 && version.minor >= 3),
        _ => true,
    }
}

pub async fn resolve_java(
    client: &reqwest::Client,
    gate: &DockerGate,
) -> Result<JavaCatalog, ResolveError> {
    let tags = gate
        .acquire()
        .await
        .list_tags(client, "eclipse-temurin")
        .await?;

    let mut versions: Vec<JavaVersion> = tags
        .iter()
        .filter_map(|tag| parse_temurin_tag(tag))
        .collect();
    versions.sort_unstable_by(|left, right| right.cmp(left));
    versions.dedup();

    if versions.is_empty() {
        return Err(ResolveError::Empty { upstream: UPSTREAM });
    }

    let mut majors: Vec<JavaMajor> = Vec::new();
    for version in versions {
        match majors.last_mut() {
            Some(entry) if entry.major == version.major() => entry.versions.push(version),
            _ => majors.push(JavaMajor {
                major: version.major(),
                versions: vec![version],
            }),
        }
    }

    Ok(JavaCatalog {
        schema: SCHEMA_VERSION,
        updated_at: Utc::now(),
        majors,
    })
}

fn parse_temurin_tag(tag: &str) -> Option<JavaVersion> {
    let candidate = tag.strip_suffix("-jdk").unwrap_or(tag);
    candidate.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::{accepted, parse_temurin_tag};
    use warehouse_common::{CatalogId, JavaVersion, Version};

    #[test]
    fn parses_both_temurin_tag_shapes() {
        assert_eq!(
            parse_temurin_tag("21.0.5_11-jdk"),
            Some(JavaVersion::Modern {
                major: 21,
                minor: 0,
                patch: 5,
                build: 11
            })
        );
        assert_eq!(
            parse_temurin_tag("8u422-b05"),
            Some(JavaVersion::Legacy {
                major: 8,
                patch: 422,
                build: 5
            })
        );
    }

    #[test]
    fn ignores_tags_that_are_not_versions() {
        for tag in ["latest", "21-jdk", "jre", "21.0.5_11-jre-alpine"] {
            assert_eq!(parse_temurin_tag(tag), None, "accepted {tag:?}");
        }
    }

    #[test]
    fn applies_per_runtime_minimums() {
        assert!(!accepted(CatalogId::Node, Version::new(10, 0, 0)));
        assert!(accepted(CatalogId::Node, Version::new(22, 1, 0)));
        assert!(!accepted(CatalogId::Python, Version::new(2, 7, 18)));
        assert!(!accepted(CatalogId::Python, Version::new(3, 2, 0)));
        assert!(accepted(CatalogId::Python, Version::new(3, 13, 1)));
        assert!(accepted(CatalogId::Rust, Version::new(1, 84, 0)));
    }
}
