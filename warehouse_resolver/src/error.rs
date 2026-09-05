use core::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    #[error("{upstream} request failed: {source}")]
    Upstream {
        upstream: &'static str,
        #[source]
        source: reqwest::Error,
    },
    #[error("{upstream} rate limited the request; retry after {}s", retry_after.as_secs())]
    RateLimited {
        upstream: &'static str,
        retry_after: Duration,
    },
    #[error("{upstream} returned a response we could not parse: {detail}")]
    Malformed {
        upstream: &'static str,
        detail: String,
    },
    #[error("{upstream} returned no usable entries")]
    Empty { upstream: &'static str },
}

impl ResolveError {
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }
}
