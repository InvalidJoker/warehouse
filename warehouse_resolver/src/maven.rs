use crate::error::ResolveError;
use crate::http;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Metadata {
    versioning: Versioning,
}

#[derive(Debug, Deserialize)]
struct Versioning {
    versions: Versions,
}

#[derive(Debug, Deserialize)]
struct Versions {
    #[serde(default)]
    version: Vec<String>,
}

pub(crate) async fn versions(
    client: &reqwest::Client,
    url: &str,
    upstream: &'static str,
) -> Result<Vec<String>, ResolveError> {
    let body = http::text(client.get(url), upstream).await?;

    let metadata: Metadata =
        quick_xml::de::from_str(&body).map_err(|err| ResolveError::Malformed {
            upstream,
            detail: format!("invalid maven metadata: {err}"),
        })?;

    Ok(metadata.versioning.versions.version)
}
