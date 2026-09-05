//! Environment-driven configuration.
//!
//! There is no configuration file. Every setting comes from a `WAREHOUSE_*` variable and
//! startup fails naming the first one that is missing or unparseable, so a
//! misconfiguration surfaces immediately rather than as a catalog that never refreshes.

use core::time::Duration;
use std::collections::BTreeMap;
use std::env;
use std::net::SocketAddr;
use std::path::PathBuf;
use warehouse_common::CatalogId;
use warehouse_common::types::catalog::ALL_CATALOGS;

/// A setting that could not be read.
#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    /// The value was present but not valid.
    #[error("{variable} is invalid: {detail}")]
    Invalid {
        /// Which variable.
        variable: String,
        /// What was wrong with it.
        detail: String,
    },
}

/// How the server runs.
#[derive(Debug, Clone)]
pub(crate) struct Config {
    /// Address to listen on. `WAREHOUSE_BIND`, default `0.0.0.0:8080`.
    pub(crate) bind: SocketAddr,
    /// Bearer token required on every route but `/health`. `WAREHOUSE_TOKEN`.
    ///
    /// Unset leaves the instance open, which is only appropriate on a trusted network.
    pub(crate) token: Option<String>,
    /// Where catalogs are persisted. `WAREHOUSE_DATA_DIR`, default `./data`.
    pub(crate) data_dir: PathBuf,
    /// User agent sent to every upstream. `WAREHOUSE_USER_AGENT`.
    ///
    /// Override it with something that identifies you and gives upstreams a way to make
    /// contact. Several of them are volunteer-run and will block traffic they cannot
    /// attribute.
    pub(crate) user_agent: String,
    /// Which catalogs to serve. `WAREHOUSE_CATALOGS`, comma separated, default all.
    pub(crate) catalogs: Vec<CatalogId>,
    /// Minimum spacing between Docker Hub tag listings. `WAREHOUSE_DOCKER_MIN_INTERVAL`.
    pub(crate) docker_min_interval: Duration,
    /// Upper bound on the random delay added to every refresh. `WAREHOUSE_REFRESH_JITTER`.
    ///
    /// Spreads the load of several instances that were started together.
    pub(crate) refresh_jitter: Duration,
    /// Delay after the first consecutive failure, doubled per failure.
    /// `WAREHOUSE_RETRY_BASE`.
    pub(crate) retry_base: Duration,
    /// Ceiling for that backoff. `WAREHOUSE_RETRY_MAX`.
    pub(crate) retry_max: Duration,
    /// How old each catalog may get before it is rebuilt.
    max_age: BTreeMap<CatalogId, Duration>,
}

const DEFAULT_USER_AGENT: &str = concat!(
    "warehouse/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/novium-dev/warehouse)"
);

impl Config {
    /// Reads the configuration from the environment.
    ///
    /// # Errors
    ///
    /// Fails on the first variable that is present but cannot be parsed.
    pub(crate) fn from_env() -> Result<Self, ConfigError> {
        let catalogs = catalogs_var()?;
        let mut max_age = BTreeMap::new();
        for id in &catalogs {
            max_age.insert(*id, max_age_var(*id)?);
        }

        Ok(Self {
            bind: parse_var("WAREHOUSE_BIND", "0.0.0.0:8080")?,
            token: env::var("WAREHOUSE_TOKEN").ok().filter(|t| !t.is_empty()),
            data_dir: env::var("WAREHOUSE_DATA_DIR")
                .unwrap_or_else(|_err| "./data".to_owned())
                .into(),
            user_agent: env::var("WAREHOUSE_USER_AGENT")
                .unwrap_or_else(|_err| DEFAULT_USER_AGENT.to_owned()),
            catalogs,
            docker_min_interval: duration_var("WAREHOUSE_DOCKER_MIN_INTERVAL", 60 * 60)?,
            refresh_jitter: duration_var("WAREHOUSE_REFRESH_JITTER", 10 * 60)?,
            retry_base: duration_var("WAREHOUSE_RETRY_BASE", 60)?,
            retry_max: duration_var("WAREHOUSE_RETRY_MAX", 60 * 60)?,
            max_age,
        })
    }

    /// How old a catalog may get before it is rebuilt.
    ///
    /// Resolved once at startup, so a refresh loop never re-reads the environment.
    pub(crate) fn max_age(&self, id: CatalogId) -> Duration {
        self.max_age
            .get(&id)
            .copied()
            .unwrap_or_else(|| default_max_age(id))
    }
}

/// Defaults follow how fast each upstream actually moves.
const fn default_max_age(id: CatalogId) -> Duration {
    Duration::from_secs(match id {
        CatalogId::Minecraft => 7 * 24 * 60 * 60,
        _ => 24 * 60 * 60,
    })
}

/// Each catalog's maximum age is overridable with `WAREHOUSE_MAX_AGE_<CATALOG>` in
/// seconds, for example `WAREHOUSE_MAX_AGE_MINECRAFT_PROXY`.
fn max_age_var(id: CatalogId) -> Result<Duration, ConfigError> {
    let variable = format!(
        "WAREHOUSE_MAX_AGE_{}",
        id.as_str().to_uppercase().replace('-', "_")
    );

    duration_var(&variable, default_max_age(id).as_secs())
}

fn parse_var<T>(variable: &str, default: &str) -> Result<T, ConfigError>
where
    T: core::str::FromStr,
    T::Err: core::fmt::Display,
{
    let raw = env::var(variable).unwrap_or_else(|_err| default.to_owned());
    raw.parse().map_err(|err| ConfigError::Invalid {
        variable: variable.to_owned(),
        detail: format!("{err} (got {raw:?})"),
    })
}

fn duration_var(variable: &str, default_secs: u64) -> Result<Duration, ConfigError> {
    let Ok(raw) = env::var(variable) else {
        return Ok(Duration::from_secs(default_secs));
    };
    raw.parse()
        .map(Duration::from_secs)
        .map_err(|err| ConfigError::Invalid {
            variable: variable.to_owned(),
            detail: format!("expected a number of seconds: {err} (got {raw:?})"),
        })
}

fn catalogs_var() -> Result<Vec<CatalogId>, ConfigError> {
    let Ok(raw) = env::var("WAREHOUSE_CATALOGS") else {
        return Ok(ALL_CATALOGS.to_vec());
    };

    let mut catalogs = Vec::new();
    for name in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
        let id = name.parse().map_err(|_err| ConfigError::Invalid {
            variable: "WAREHOUSE_CATALOGS".to_owned(),
            detail: format!(
                "unknown catalog {name:?}; known catalogs are {}",
                ALL_CATALOGS
                    .iter()
                    .map(|catalog| catalog.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        })?;
        if !catalogs.contains(&id) {
            catalogs.push(id);
        }
    }

    if catalogs.is_empty() {
        return Err(ConfigError::Invalid {
            variable: "WAREHOUSE_CATALOGS".to_owned(),
            detail: "no catalogs selected".to_owned(),
        });
    }

    Ok(catalogs)
}
