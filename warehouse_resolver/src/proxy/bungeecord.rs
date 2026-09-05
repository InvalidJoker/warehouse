//! BungeeCord builds, served by SpigotMC's Jenkins.

use crate::error::ResolveError;
use crate::http;
use serde::Deserialize;

const UPSTREAM: &str = "spigotmc";
const JOB: &str = "BungeeCord";
const RESULT_SUCCESS: &str = "SUCCESS";

/// How many recent builds to keep. Jenkins holds far more than anyone would pick from.
const MAX_BUILDS: usize = 50;

#[derive(Debug, Deserialize)]
struct Job {
    builds: Vec<Build>,
}

#[derive(Debug, Deserialize)]
struct Build {
    number: u16,
    #[serde(default)]
    result: Option<String>,
}

/// Resolves successful BungeeCord build numbers, newest first.
pub(crate) async fn resolve(
    client: &reqwest::Client,
    base_url: &str,
) -> Result<Vec<u16>, ResolveError> {
    let job: Job = http::json(
        client
            .get(format!("{base_url}job/{JOB}/api/json"))
            .query(&[("tree", "builds[number,result]")]),
        UPSTREAM,
    )
    .await?;

    let mut builds: Vec<u16> = job
        .builds
        .into_iter()
        .filter(|build| build.result.as_deref() == Some(RESULT_SUCCESS))
        .map(|build| build.number)
        .collect();
    builds.sort_unstable_by(|left, right| right.cmp(left));
    builds.truncate(MAX_BUILDS);

    if builds.is_empty() {
        return Err(ResolveError::Empty { upstream: UPSTREAM });
    }

    Ok(builds)
}
