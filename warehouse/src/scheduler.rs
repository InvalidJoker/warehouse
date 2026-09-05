use crate::state::{Catalog, Warehouse};
use crate::store::Held;
use chrono::Utc;
use core::time::Duration;
use rand::Rng as _;
use std::sync::Arc;
use warehouse_common::{CatalogId, SCHEMA_VERSION};

pub(crate) fn spawn(warehouse: &Warehouse) {
    for catalog in warehouse.catalogs.values() {
        let warehouse = warehouse.clone();
        let catalog = Arc::clone(catalog);
        tokio::spawn(async move { run(warehouse, catalog).await });
    }
}

async fn run(warehouse: Warehouse, catalog: Arc<Catalog>) {
    loop {
        let delay = due_in(&warehouse, &catalog).await;

        if !delay.is_zero() {
            let next = Utc::now() + chrono::TimeDelta::from_std(delay).unwrap_or_default();
            catalog
                .update_status(|status| status.next_attempt = Some(next))
                .await;

            tracing::debug!(
                catalog = %catalog.id,
                seconds = delay.as_secs(),
                "next refresh scheduled"
            );

            tokio::select! {
                () = tokio::time::sleep(delay) => {}
                () = catalog.refresh_requested() => {
                    tracing::info!(catalog = %catalog.id, "refresh requested");
                }
            }
        }

        refresh(&warehouse, &catalog).await;
    }
}

async fn due_in(warehouse: &Warehouse, catalog: &Catalog) -> Duration {
    let status = catalog.status().await;

    if status.consecutive_failures > 0 {
        return jitter(
            backoff(warehouse, status.consecutive_failures),
            warehouse.config.refresh_jitter,
        );
    }

    let Some(held) = catalog.held().await else {
        return Duration::ZERO;
    };

    let max_age = warehouse.config.max_age(catalog.id);
    let age = Utc::now().signed_duration_since(held.updated_at);
    let age = age.to_std().unwrap_or(Duration::ZERO);

    jitter(max_age.saturating_sub(age), warehouse.config.refresh_jitter)
}

fn backoff(warehouse: &Warehouse, failures: u32) -> Duration {
    let exponent = failures.saturating_sub(1).min(16);
    warehouse
        .config
        .retry_base
        .saturating_mul(1u32 << exponent)
        .min(warehouse.config.retry_max)
}

fn jitter(base: Duration, spread: Duration) -> Duration {
    if spread.is_zero() {
        return base;
    }
    let extra = rand::rng().random_range(0..=spread.as_secs());
    base.saturating_add(Duration::from_secs(extra))
}

async fn refresh(warehouse: &Warehouse, catalog: &Catalog) {
    let id = catalog.id;
    tracing::info!(catalog = %id, "refreshing catalog");

    let resolved = warehouse_resolver::resolve(&warehouse.client, &warehouse.docker, id).await;

    let document = match resolved {
        Ok(document) => document,
        Err(err) => {
            tracing::warn!(catalog = %id, error = %err, "refresh failed");
            let message = err.to_string();
            catalog
                .update_status(|status| {
                    status.consecutive_failures = status.consecutive_failures.saturating_add(1);
                    status.last_failure = Some(Utc::now());
                    status.last_error = Some(message);
                })
                .await;
            return;
        }
    };

    let held = match Held::new(document) {
        Ok(held) => held,
        Err(err) => {
            tracing::error!(catalog = %id, error = %err, "could not encode resolved catalog");
            catalog
                .update_status(|status| {
                    status.consecutive_failures = status.consecutive_failures.saturating_add(1);
                    status.last_failure = Some(Utc::now());
                    status.last_error = Some(format!("encoding failed: {err}"));
                })
                .await;
            return;
        }
    };

    let updated_at = held.updated_at;
    let etag = held.etag.clone();
    catalog.store(held.clone()).await;

    if let Err(err) = warehouse.store.save(id, SCHEMA_VERSION, &held).await {
        tracing::error!(catalog = %id, error = %err, "could not persist catalog");
    }

    catalog
        .update_status(|status| {
            status.consecutive_failures = 0;
            status.last_error = None;
            status.last_success = Some(updated_at);
        })
        .await;

    tracing::info!(catalog = %id, %etag, "catalog refreshed");
}

pub(crate) async fn restore(warehouse: &Warehouse) {
    for (id, catalog) in warehouse.catalogs.iter() {
        let Some(held) = warehouse.store.load(*id, SCHEMA_VERSION).await else {
            continue;
        };

        tracing::info!(
            catalog = %id,
            updated_at = %held.updated_at,
            "restored catalog from disk"
        );
        catalog
            .update_status(|status| status.last_success = Some(held.updated_at))
            .await;
        catalog.store(held).await;
    }
}

pub(crate) async fn missing(warehouse: &Warehouse) -> Vec<CatalogId> {
    let mut missing = Vec::new();
    for (id, catalog) in warehouse.catalogs.iter() {
        if catalog.held().await.is_none() {
            missing.push(*id);
        }
    }
    missing
}
