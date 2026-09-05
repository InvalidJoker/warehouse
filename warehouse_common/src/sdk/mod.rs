//! A ready client for a Warehouse instance.
//!
//! The SDK is generated from the same service definitions the server implements, so a
//! route cannot drift between the two. Its methods are the trait methods in
//! [`crate::service`].
//!
//! Warehouse is not meant to sit on your request path. Fetch catalogs in the background,
//! hold them in memory, and keep serving the last ones you got when the instance is
//! unreachable — a stale catalog is almost always better than an error.

use crate::SCHEMA_VERSION;
use crate::service::catalog::CatalogServiceClientImpl;
use crate::service::system::SystemServiceClientImpl;
use crate::service::{
    WarehouseServices, WarehouseServicesClientImpl, WarehouseServicesURL, ZelusClientImpl,
};
use zelus::reqwest::Client;
use zelus::reqwest::header::{HeaderMap, HeaderValue};
use zelus::url::Url;

/// The client could not be constructed.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// The token contained bytes that cannot go in a header.
    #[error("the token is not a valid header value")]
    Token,
    /// The HTTP client could not be built.
    #[error("could not build the http client: {0}")]
    Client(#[from] zelus::reqwest::Error),
}

/// A client for one Warehouse instance.
#[derive(Debug, Clone)]
#[must_use]
pub struct WarehouseSDK {
    pub(crate) client: Client,
    pub(crate) base_url: Url,
}

impl WarehouseSDK {
    /// Builds a client against `base_url`, authenticating with `token` when the instance
    /// requires one.
    ///
    /// # Errors
    ///
    /// Fails if the token cannot be sent as a header, or the HTTP client cannot be built.
    pub fn new(base_url: Url, token: Option<&str>) -> Result<Self, BuildError> {
        let mut headers = HeaderMap::new();
        if let Some(token) = token {
            let mut value = HeaderValue::from_str(&format!("Bearer {token}"))
                .map_err(|_err| BuildError::Token)?;
            value.set_sensitive(true);
            headers.insert(zelus::reqwest::header::AUTHORIZATION, value);
        }

        let client = Client::builder()
            .user_agent(concat!("warehouse-sdk/", env!("CARGO_PKG_VERSION")))
            .default_headers(headers)
            .timeout(core::time::Duration::from_secs(30))
            .build()?;

        Ok(Self::with_client(client, base_url))
    }

    /// Builds a client reusing an existing HTTP client, which must already carry whatever
    /// authentication the instance requires.
    pub fn with_client(client: Client, base_url: Url) -> Self {
        Self { client, base_url }
    }

    /// The schema version this SDK was compiled against.
    ///
    /// Compare it against the `schema` field of a fetched manifest before trusting any
    /// document: an instance serving a different schema may return bodies this SDK would
    /// misread.
    #[must_use]
    pub const fn schema(&self) -> u32 {
        SCHEMA_VERSION
    }
}

impl ZelusClientImpl for WarehouseSDK {
    fn client(&self) -> &Client {
        &self.client
    }

    fn base_url(&self) -> &Url {
        &self.base_url
    }
}

impl CatalogServiceClientImpl for WarehouseSDK {}
impl SystemServiceClientImpl for WarehouseSDK {}
impl WarehouseServicesClientImpl for WarehouseSDK {}
impl WarehouseServices for WarehouseSDK {}
impl WarehouseServicesURL for WarehouseSDK {}
