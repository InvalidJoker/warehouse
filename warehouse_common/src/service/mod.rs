use crate::service::catalog::{CatalogService, CatalogServiceClientImpl, CatalogServiceURL};
use crate::service::system::{SystemService, SystemServiceClientImpl, SystemServiceURL};

pub(crate) use zelus::define_path_variable;
pub(crate) use zelus::error::{define_error, error};

pub use zelus::framework_router;
pub use zelus::sdk::ZelusClientImpl;

pub mod catalog;
pub mod system;

define_error!(auth {
    invalid("Authentication is invalid" UNAUTHORIZED),
});

define_error!(catalog {
    not_found("This instance does not serve that catalog" NOT_FOUND),
    unavailable("That catalog has not been resolved yet" SERVICE_UNAVAILABLE),
});

define_path_variable!(catalog "The catalog id");
define_path_variable!(runtime "The language runtime catalog id");

error!(BlankError, auth::invalid);

pub trait WarehouseServices: CatalogService + SystemService {}

pub trait WarehouseServicesClientImpl: CatalogServiceClientImpl + SystemServiceClientImpl {}

pub trait WarehouseServicesURL: WarehouseServices + CatalogServiceURL + SystemServiceURL {}

#[must_use]
pub fn api_document<T>(instance: &T) -> utoipa::openapi::OpenApi
where
    T: WarehouseServices + Clone + Send + Sync + Sized + 'static,
{
    use utoipa::OpenApi as _;

    #[derive(utoipa::OpenApi)]
    #[openapi()]
    struct ApiDoc;

    let (with_auth, without_auth) = framework_router!(T instance (with_auth,without_auth,) {
        CatalogService, SystemService
    });

    let (_router, api) = utoipa_axum::router::OpenApiRouter::with_openapi(ApiDoc::openapi())
        .merge(with_auth.into_openapi())
        .merge(without_auth.into_openapi())
        .split_for_parts();

    api
}
