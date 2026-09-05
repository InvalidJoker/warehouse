use core::net::SocketAddr;
use core::time::Duration;
use std::collections::BTreeMap;
use std::env;
use std::path::PathBuf;
use warehouse_common::CatalogId;
use warehouse_common::types::catalog::ALL_CATALOGS;

#[derive(Debug, thiserror::Error)]
pub(crate) enum ConfigError {
    #[error("{variable} is invalid: {detail}")]
    Invalid { variable: String, detail: String },
}

#[derive(Debug, Clone)]
pub(crate) struct Config {
    pub(crate) bind: SocketAddr,
    pub(crate) token: Option<String>,
    pub(crate) data_dir: PathBuf,
    pub(crate) user_agent: String,
    pub(crate) catalogs: Vec<CatalogId>,
    pub(crate) docker_min_interval: Duration,
    pub(crate) refresh_jitter: Duration,
    pub(crate) retry_base: Duration,
    pub(crate) retry_max: Duration,
    max_age: BTreeMap<CatalogId, Duration>,
}

const DEFAULT_USER_AGENT: &str = concat!(
    "warehouse/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/InvalidJoker/warehouse)"
);

impl Config {
    pub(crate) fn from_env() -> Result<Self, ConfigError> {
        let catalogs = catalogs_var()?;
        let mut max_age = BTreeMap::new();
        for id in &catalogs {
            max_age.insert(*id, max_age_var(*id)?);
        }

        Ok(Self {
            bind: parse_var("WAREHOUSE_BIND", "0.0.0.0:8080")?,
            token: env::var("WAREHOUSE_TOKEN")
                .ok()
                .filter(|token| !token.is_empty()),
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

    pub(crate) fn max_age(&self, id: CatalogId) -> Duration {
        self.max_age
            .get(&id)
            .copied()
            .unwrap_or_else(|| default_max_age(id))
    }
}

const fn default_max_age(id: CatalogId) -> Duration {
    Duration::from_secs(match id {
        CatalogId::Minecraft => 7 * 24 * 60 * 60,
        _ => 24 * 60 * 60,
    })
}

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
    for name in raw
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
    {
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
