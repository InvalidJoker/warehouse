use crate::state::Warehouse;
use axum::Router;
use axum::extract::Request;
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use sha2::{Digest as _, Sha256};
use tower_http::trace::TraceLayer;
use utoipa::OpenApi as _;
use utoipa_axum::router::OpenApiRouter;
use warehouse_common::service::catalog::CatalogService;
use warehouse_common::service::catalog::catalog_service_error::{
    JavaCatalogError, ManifestError, MinecraftCatalogError, ProxyCatalogError, RefreshCatalogError,
    RuntimeCatalogError,
};
use warehouse_common::service::system::SystemService;
use warehouse_common::service::system::system_service_error::{HealthError, StatusError};
use warehouse_common::service::{BlankError, WarehouseServices, framework_router};
use warehouse_common::types::status::{CatalogStatus, StatusReport};
use warehouse_common::{
    CatalogEntry, CatalogId, JavaCatalog, Manifest, MinecraftCatalog, ProxyCatalog, RuntimeCatalog,
    SCHEMA_VERSION,
};

impl WarehouseServices for Warehouse {}

async fn authenticate(request: Request, next: Next) -> Response {
    let Some(warehouse) = request.extensions().get::<Warehouse>().cloned() else {
        tracing::error!("request reached the auth layer without shared state");
        return BlankError::AuthInvalid.into_response();
    };

    let Some(expected) = warehouse.config.token.as_deref() else {
        return next.run(request).await;
    };

    let presented = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();

    if Sha256::digest(presented.as_bytes()) == Sha256::digest(expected.as_bytes()) {
        return next.run(request).await;
    }

    BlankError::AuthInvalid.into_response()
}

#[zelus::async_trait]
impl SystemService for Warehouse {
    async fn health(&self) -> Result<(), HealthError> {
        Ok(())
    }

    async fn status(&self) -> Result<StatusReport, StatusError> {
        let mut catalogs = Vec::new();
        for entry in self.catalogs.values() {
            let status = entry.status().await;
            catalogs.push(CatalogStatus {
                id: Some(entry.id),
                resolved: entry.held().await.is_some(),
                stale: status.is_stale(),
                last_success: status.last_success,
                last_failure: status.last_failure,
                last_error: status.last_error,
                consecutive_failures: status.consecutive_failures,
                next_attempt: status.next_attempt,
            });
        }

        Ok(StatusReport {
            schema: SCHEMA_VERSION,
            version: env!("CARGO_PKG_VERSION").to_owned(),
            catalogs,
        })
    }
}

#[zelus::async_trait]
impl CatalogService for Warehouse {
    async fn manifest(&self) -> Result<Manifest, ManifestError> {
        let mut catalogs = Vec::new();
        for entry in self.catalogs.values() {
            let Some(held) = entry.held().await else {
                continue;
            };
            catalogs.push(CatalogEntry {
                id: entry.id,
                etag: held.etag,
                updated_at: held.updated_at,
                stale: entry.status().await.is_stale(),
            });
        }

        Ok(Manifest {
            schema: SCHEMA_VERSION,
            catalogs,
        })
    }

    async fn minecraft_catalog(&self) -> Result<MinecraftCatalog, MinecraftCatalogError> {
        self.document(CatalogId::Minecraft)
            .await
            .and_then(|document| document.as_minecraft().cloned())
            .ok_or(MinecraftCatalogError::CatalogUnavailable)
    }

    async fn proxy_catalog(&self) -> Result<ProxyCatalog, ProxyCatalogError> {
        self.document(CatalogId::MinecraftProxy)
            .await
            .and_then(|document| document.as_proxy().cloned())
            .ok_or(ProxyCatalogError::CatalogUnavailable)
    }

    async fn java_catalog(&self) -> Result<JavaCatalog, JavaCatalogError> {
        self.document(CatalogId::Java)
            .await
            .and_then(|document| document.as_java().cloned())
            .ok_or(JavaCatalogError::CatalogUnavailable)
    }

    async fn runtime_catalog(
        &self,
        runtime: CatalogId,
    ) -> Result<RuntimeCatalog, RuntimeCatalogError> {
        if !runtime.uses_docker_hub() || runtime == CatalogId::Java {
            return Err(RuntimeCatalogError::CatalogNotFound);
        }
        if self.catalog(runtime).is_none() {
            return Err(RuntimeCatalogError::CatalogNotFound);
        }

        self.document(runtime)
            .await
            .and_then(|document| document.as_runtime().cloned())
            .ok_or(RuntimeCatalogError::CatalogUnavailable)
    }

    async fn refresh_catalog(&self, catalog: CatalogId) -> Result<(), RefreshCatalogError> {
        let entry = self
            .catalog(catalog)
            .ok_or(RefreshCatalogError::CatalogNotFound)?;

        entry.request_refresh();
        Ok(())
    }
}

impl Warehouse {
    async fn document(
        &self,
        id: CatalogId,
    ) -> Option<std::sync::Arc<warehouse_common::types::document::Document>> {
        Some(self.catalog(id)?.held().await?.document)
    }

    pub(crate) fn router(&self) -> Router {
        #[derive(utoipa::OpenApi)]
        #[openapi()]
        struct ApiDoc;

        let (authenticated, public) = framework_router!(Self self (with_auth,without_auth,) {
            CatalogService, SystemService
        });

        let authenticated = authenticated
            .into_openapi()
            .layer(axum::middleware::from_fn(authenticate));

        let (router, api) = OpenApiRouter::with_openapi(ApiDoc::openapi())
            .merge(authenticated)
            .merge(public.into_openapi())
            .split_for_parts();

        router
            .merge(utoipa_swagger_ui::SwaggerUi::new("/docs").url("/docs/openapi.json", api))
            .layer(axum::Extension(self.clone()))
            .layer(TraceLayer::new_for_http())
    }
}
