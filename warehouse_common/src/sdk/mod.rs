use crate::SCHEMA_VERSION;
use crate::service::catalog::CatalogServiceClientImpl;
use crate::service::system::SystemServiceClientImpl;
use crate::service::{
    WarehouseServices, WarehouseServicesClientImpl, WarehouseServicesURL, ZelusClientImpl,
};
use zelus::reqwest::Client;
use zelus::reqwest::header::{HeaderMap, HeaderValue};
use zelus::url::Url;

#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("the token is not a valid header value")]
    Token,
    #[error("could not build the http client: {0}")]
    Client(#[from] zelus::reqwest::Error),
}

#[derive(Debug, Clone)]
#[must_use]
pub struct WarehouseSDK {
    pub(crate) client: Client,
    pub(crate) base_url: Url,
}

impl WarehouseSDK {
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

    pub const fn with_client(client: Client, base_url: Url) -> Self {
        Self { client, base_url }
    }

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
