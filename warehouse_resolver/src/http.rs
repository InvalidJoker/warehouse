//! Shared HTTP plumbing for the resolvers.

use crate::error::ResolveError;
use core::time::Duration;
use serde::de::DeserializeOwned;

/// Builds the HTTP client the resolvers share.
///
/// The user agent is mandatory and should identify the operator: several of the
/// upstreams Warehouse reads are small volunteer-run projects, and an anonymous
/// scraper is the first thing they block.
///
/// # Errors
///
/// Fails if the underlying TLS or connection pool cannot be constructed.
pub fn client(user_agent: &str) -> Result<reqwest::Client, reqwest::Error> {
    reqwest::Client::builder()
        .user_agent(user_agent)
        .timeout(Duration::from_secs(60))
        .connect_timeout(Duration::from_secs(10))
        .build()
}

pub(crate) fn upstream_error(upstream: &'static str) -> impl Fn(reqwest::Error) -> ResolveError {
    move |source| ResolveError::Upstream { upstream, source }
}

pub(crate) fn retry_after(response: &reqwest::Response) -> Duration {
    response
        .headers()
        .get(reqwest::header::RETRY_AFTER)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(Duration::from_secs(60), Duration::from_secs)
}

/// Sends a request and rejects rate limits and error statuses.
pub(crate) async fn send(
    request: reqwest::RequestBuilder,
    upstream: &'static str,
) -> Result<reqwest::Response, ResolveError> {
    let response = request.send().await.map_err(upstream_error(upstream))?;

    if response.status() == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(ResolveError::RateLimited {
            upstream,
            retry_after: retry_after(&response),
        });
    }

    response
        .error_for_status()
        .map_err(upstream_error(upstream))
}

/// Sends a request and decodes the JSON body, keeping the body in the error on failure.
///
/// Upstream APIs change shape without warning, and a bare serde error names a path but
/// not the payload it failed on. Carrying a bounded prefix of the body into the error
/// turns "missing field `versions`" into something an operator can act on.
pub(crate) async fn json<T: DeserializeOwned>(
    request: reqwest::RequestBuilder,
    upstream: &'static str,
) -> Result<T, ResolveError> {
    let body = send(request, upstream)
        .await?
        .bytes()
        .await
        .map_err(upstream_error(upstream))?;

    serde_json::from_slice(&body).map_err(|err| ResolveError::Malformed {
        upstream,
        detail: format!("{err}; body started with {}", preview(&body)),
    })
}

/// Sends a request and returns the body as text.
pub(crate) async fn text(
    request: reqwest::RequestBuilder,
    upstream: &'static str,
) -> Result<String, ResolveError> {
    send(request, upstream)
        .await?
        .text()
        .await
        .map_err(upstream_error(upstream))
}

const PREVIEW_LIMIT: usize = 256;

fn preview(body: &[u8]) -> String {
    let end = body.len().min(PREVIEW_LIMIT);
    let truncated = String::from_utf8_lossy(&body[..end]).into_owned();
    if body.len() > end {
        format!("{truncated}…")
    } else {
        truncated
    }
}
