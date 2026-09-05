use crate::config::Config;
use crate::store::{Held, Store};
use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use warehouse_common::CatalogId;
use warehouse_resolver::DockerGate;

#[derive(Debug, Clone, Default)]
pub(crate) struct Status {
    pub(crate) last_success: Option<DateTime<Utc>>,
    pub(crate) last_failure: Option<DateTime<Utc>>,
    pub(crate) last_error: Option<String>,
    pub(crate) consecutive_failures: u32,
    pub(crate) next_attempt: Option<DateTime<Utc>>,
}

impl Status {
    pub(crate) const fn is_stale(&self) -> bool {
        self.consecutive_failures > 0
    }
}

#[derive(Debug)]
pub(crate) struct Catalog {
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

    pub(crate) async fn held(&self) -> Option<Held> {
        self.held.read().await.clone()
    }

    pub(crate) async fn store(&self, held: Held) {
        *self.held.write().await = Some(held);
    }

    pub(crate) async fn status(&self) -> Status {
        self.status.read().await.clone()
    }

    pub(crate) async fn update_status(&self, update: impl FnOnce(&mut Status)) {
        update(&mut *self.status.write().await);
    }

    pub(crate) fn request_refresh(&self) {
        self.refresh.notify_one();
    }

    pub(crate) async fn refresh_requested(&self) {
        self.refresh.notified().await;
    }
}

#[derive(Debug, Clone)]
pub(crate) struct Warehouse {
    pub(crate) config: Arc<Config>,
    pub(crate) catalogs: Arc<BTreeMap<CatalogId, Arc<Catalog>>>,
    pub(crate) store: Store,
    pub(crate) client: reqwest::Client,
    pub(crate) docker: Arc<DockerGate>,
}

impl Warehouse {
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

    #[must_use]
    pub(crate) fn catalog(&self, id: CatalogId) -> Option<&Arc<Catalog>> {
        self.catalogs.get(&id)
    }
}
