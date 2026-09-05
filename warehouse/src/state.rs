//! Shared server state.

use crate::config::Config;
use crate::store::{Held, Store};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use warehouse_common::CatalogId;
use warehouse_resolver::DockerGate;

/// What happened on the recent refresh attempts of one catalog.
///
/// Mirrors [`warehouse_common::CatalogStatus`] minus the fields the server fills in when
/// it reports, so the refresh loop can update it without rebuilding the public shape.
#[derive(Debug, Clone, Default)]
pub(crate) struct Status {
    /// When the catalog was last rebuilt successfully.
    pub(crate) last_success: Option<DateTime<Utc>>,
    /// When the last attempt failed.
    pub(crate) last_failure: Option<DateTime<Utc>>,
    /// The error from that failure, for an operator reading `/system/status`.
    pub(crate) last_error: Option<String>,
    /// Failures since the last success. Drives the retry backoff.
    pub(crate) consecutive_failures: u32,
    /// When the next refresh is currently scheduled for.
    pub(crate) next_attempt: Option<DateTime<Utc>>,
}

impl Status {
    /// Whether the last attempt failed.
    pub(crate) const fn is_stale(&self) -> bool {
        self.consecutive_failures > 0
    }
}

/// One catalog's held document, status, and wake-up signal.
#[derive(Debug)]
pub(crate) struct Catalog {
    /// Which catalog this is.
    pub(crate) id: CatalogId,
    held: RwLock<Option<Held>>,
    status: RwLock<Status>,
    refresh: Notify,
}

impl Catalog {
    fn new(id: CatalogId) -> Self {
        Self {
            id,
            held: RwLock::new(None),
            status: RwLock::new(Status::default()),
            refresh: Notify::new(),
        }
    }

    /// The document currently being served, if any has ever been resolved.
    pub(crate) async fn held(&self) -> Option<Held> {
        self.held.read().await.clone()
    }

    /// Replaces the served document.
    pub(crate) async fn store(&self, held: Held) {
        *self.held.write().await = Some(held);
    }

    /// A snapshot of the refresh status.
    pub(crate) async fn status(&self) -> Status {
        self.status.read().await.clone()
    }

    /// Updates the refresh status.
    pub(crate) async fn update_status(&self, update: impl FnOnce(&mut Status)) {
        update(&mut *self.status.write().await);
    }

    /// Asks the refresh loop to run now instead of waiting for its next tick.
    pub(crate) fn request_refresh(&self) {
        self.refresh.notify_one();
    }

    /// Waits for a forced refresh request.
    pub(crate) async fn refresh_requested(&self) {
        self.refresh.notified().await;
    }
}

/// Everything the request handlers and refresh loops share.
#[derive(Debug, Clone)]
pub(crate) struct Warehouse {
    /// The loaded configuration.
    pub(crate) config: Arc<Config>,
    /// Catalogs this instance serves, in configuration order.
    pub(crate) catalogs: Arc<BTreeMap<CatalogId, Arc<Catalog>>>,
    /// Where catalogs are persisted.
    pub(crate) store: Store,
    /// HTTP client shared by every resolver.
    pub(crate) client: reqwest::Client,
    /// Gate serializing Docker Hub reads.
    pub(crate) docker: Arc<DockerGate>,
}

impl Warehouse {
    /// Builds the shared state for the configured catalogs.
    #[must_use]
    pub(crate) fn new(config: Arc<Config>, store: Store, client: reqwest::Client) -> Self {
        let catalogs = config
            .catalogs
            .iter()
            .map(|id| (*id, Arc::new(Catalog::new(*id))))
            .collect();

        let docker = Arc::new(DockerGate::new(config.docker_min_interval));

        Self {
            config,
            catalogs: Arc::new(catalogs),
            store,
            client,
            docker,
        }
    }

    /// Looks up a catalog this instance serves.
    #[must_use]
    pub(crate) fn catalog(&self, id: CatalogId) -> Option<&Arc<Catalog>> {
        self.catalogs.get(&id)
    }
}
