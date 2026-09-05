mod config;
mod scheduler;
mod service;
mod state;
mod store;

use crate::config::Config;
use crate::state::Warehouse;
use crate::store::Store;
use std::process::ExitCode;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_env("WAREHOUSE_LOG")
                .unwrap_or_else(|_err| EnvFilter::new("warehouse=info,warehouse_resolver=info")),
        )
        .init();

    let runtime = match tokio::runtime::Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            tracing::error!(error = %err, "could not start the async runtime");
            return ExitCode::FAILURE;
        }
    };

    if std::env::args().nth(1).as_deref() == Some("healthcheck") {
        return runtime.block_on(healthcheck());
    }

    match runtime.block_on(serve()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            tracing::error!(error = %err, "warehouse stopped");
            ExitCode::FAILURE
        }
    }
}

async fn healthcheck() -> ExitCode {
    let bind = std::env::var("WAREHOUSE_BIND").unwrap_or_else(|_err| "0.0.0.0:8080".to_owned());
    let port = bind.rsplit(':').next().unwrap_or("8080");
    let url = format!("http://127.0.0.1:{port}/system/health");

    match reqwest::get(&url).await {
        Ok(response) if response.status().is_success() => ExitCode::SUCCESS,
        Ok(response) => {
            tracing::error!(status = %response.status(), "health check failed");
            ExitCode::FAILURE
        }
        Err(err) => {
            tracing::error!(error = %err, "health check could not reach the instance");
            ExitCode::FAILURE
        }
    }
}

#[derive(Debug, thiserror::Error)]
enum StartupError {
    #[error(transparent)]
    Config(#[from] config::ConfigError),
    #[error("could not open the data directory: {0}")]
    Store(#[source] std::io::Error),
    #[error("could not build the upstream http client: {0}")]
    Client(#[source] reqwest::Error),
    #[error("could not bind {address}: {source}")]
    Bind {
        address: core::net::SocketAddr,
        #[source]
        source: std::io::Error,
    },
    #[error("server error: {0}")]
    Serve(#[source] std::io::Error),
}

async fn serve() -> Result<(), StartupError> {
    let config = Arc::new(Config::from_env()?);

    let store = Store::open(&config.data_dir)
        .await
        .map_err(StartupError::Store)?;
    let client = warehouse_resolver::client(&config.user_agent).map_err(StartupError::Client)?;

    let warehouse = Warehouse::new(Arc::clone(&config), store, client);

    scheduler::restore(&warehouse).await;
    let missing = scheduler::missing(&warehouse).await;
    if missing.is_empty() {
        tracing::info!("all configured catalogs restored from disk");
    } else {
        tracing::info!(
            catalogs = ?missing.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            "these catalogs will be resolved on startup"
        );
    }

    scheduler::spawn(&warehouse);

    let listener = tokio::net::TcpListener::bind(config.bind)
        .await
        .map_err(|source| StartupError::Bind {
            address: config.bind,
            source,
        })?;

    tracing::info!(
        address = %config.bind,
        authenticated = config.token.is_some(),
        "warehouse listening"
    );
    if config.token.is_none() {
        tracing::warn!("WAREHOUSE_TOKEN is unset: every catalog is served without authentication");
    }

    axum::serve(listener, warehouse.router())
        .with_graceful_shutdown(shutdown())
        .await
        .map_err(StartupError::Serve)
}

async fn shutdown() {
    let interrupt = tokio::signal::ctrl_c();

    #[cfg(unix)]
    {
        let mut terminate =
            match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
                Ok(signal) => signal,
                Err(err) => {
                    tracing::warn!(error = %err, "cannot listen for SIGTERM");
                    let _ = interrupt.await;
                    return;
                }
            };

        tokio::select! {
            _ = interrupt => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    let _ = interrupt.await;

    tracing::info!("shutting down");
}
