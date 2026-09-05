//! Errors produced while rebuilding a catalog from upstream.

use core::time::Duration;

/// Why a catalog could not be rebuilt.
///
/// Every variant names the upstream that failed, so an operator reading `/status` can
/// tell a Mojang outage from a Docker Hub rate limit without reading logs.
#[derive(Debug, thiserror::Error)]
pub enum ResolveError {
    /// The upstream could not be reached, or answered with an error status.
    #[error("{upstream} request failed: {source}")]
    Upstream {
        /// Which upstream failed.
        upstream: &'static str,
        /// The underlying transport or status error.
        #[source]
        source: reqwest::Error,
    },
    /// The upstream asked us to slow down.
    #[error("{upstream} rate limited the request; retry after {}s", retry_after.as_secs())]
    RateLimited {
        /// Which upstream rate limited us.
        upstream: &'static str,
        /// How long the upstream asked us to wait.
        retry_after: Duration,
    },
    /// The upstream answered, but not with anything we could read.
    #[error("{upstream} returned a response we could not parse: {detail}")]
    Malformed {
        /// Which upstream returned the bad response.
        upstream: &'static str,
        /// What went wrong.
        detail: String,
    },
    /// The upstream answered successfully but with no usable data.
    ///
    /// Treated as a failure so a catalog is never replaced with an empty document.
    #[error("{upstream} returned no usable entries")]
    Empty {
        /// Which upstream came back empty.
        upstream: &'static str,
    },
}

impl ResolveError {
    /// How long to wait before retrying, when the upstream said.
    #[must_use]
    pub const fn retry_after(&self) -> Option<Duration> {
        match self {
            Self::RateLimited { retry_after, .. } => Some(*retry_after),
            _ => None,
        }
    }
}
