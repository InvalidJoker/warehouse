use crate::error::ResolveError;
use crate::http;
use core::time::Duration;
use serde::Deserialize;
use std::collections::BTreeSet;
use tokio::sync::Mutex;
use tokio::time::Instant;
use url::Url;

const UPSTREAM: &str = "dockerhub";

const MAX_PAGES: usize = 5;

#[derive(Debug, Deserialize)]
struct TagsResponse {
    next: Option<Url>,
    results: Vec<Tag>,
}

#[derive(Debug, Deserialize)]
struct Tag {
    name: String,
}

#[derive(Debug)]
pub struct DockerGate {
    min_interval: Duration,
    last: Mutex<Option<Instant>>,
}

impl DockerGate {
    #[must_use]
    pub const fn new(min_interval: Duration) -> Self {
        Self {
            min_interval,
            last: Mutex::const_new(None),
        }
    }

    pub async fn acquire(&self) -> DockerPermit<'_> {
        let last = self.last.lock().await;

        if let Some(previous) = *last {
            let elapsed = previous.elapsed();
            if elapsed < self.min_interval {
                let wait = self.min_interval.saturating_sub(elapsed);
                tracing::debug!(wait_secs = wait.as_secs(), "waiting for docker hub gate");
                tokio::time::sleep(wait).await;
            }
        }

        DockerPermit { last }
    }
}

#[derive(Debug)]
pub struct DockerPermit<'gate> {
    last: tokio::sync::MutexGuard<'gate, Option<Instant>>,
}

impl DockerPermit<'_> {
    pub async fn list_tags(
        mut self,
        client: &reqwest::Client,
        image: &str,
    ) -> Result<BTreeSet<String>, ResolveError> {
        *self.last = Some(Instant::now());

        let first = format!(
            "https://hub.docker.com/v2/repositories/library/{image}/tags?page_size=50&ordering=last_updated"
        );
        let mut next = Some(Url::parse(&first).map_err(|err| ResolveError::Malformed {
            upstream: UPSTREAM,
            detail: format!("could not build tag listing url for {image}: {err}"),
        })?);

        let mut tags = BTreeSet::new();
        for _ in 0..MAX_PAGES {
            let Some(url) = next.take() else {
                break;
            };

            let page: TagsResponse = http::json(client.get(url), UPSTREAM).await?;
            tags.extend(page.results.into_iter().map(|tag| tag.name));
            next = page.next;
        }

        if tags.is_empty() {
            return Err(ResolveError::Empty { upstream: UPSTREAM });
        }

        Ok(tags)
    }
}
