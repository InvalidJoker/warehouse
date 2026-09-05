//! Schema, service definitions and SDK for the Warehouse version catalog service.
//!
//! Warehouse aggregates release metadata from upstream projects (Mojang, PaperMC,
//! FabricMC, NeoForged, Docker Hub and others) and republishes it as a small set of
//! stable JSON documents called *catalogs*.
//!
//! This crate is what consumers depend on. With the default `service` feature it carries
//! the zelus service traits, which both the server and the OpenAPI document are built
//! from; with `sdk` it also carries [`sdk::WarehouseSDK`], a ready client.
//!
//! # The identifier invariant
//!
//! Catalogs contain **version identifiers only** — never download URLs, container image
//! references, checksums or any other value a consumer might fetch or execute. Consumers
//! build those themselves from constants they control. This keeps a Warehouse instance
//! outside the trust boundary of whatever it feeds: a compromised or malicious instance
//! can offer a consumer a version that does not exist, but it can never redirect that
//! consumer to an attacker-controlled artifact.
//!
//! Any change that would place a fetchable location in a catalog breaks this contract and
//! must be rejected.

pub mod types;

#[cfg(feature = "service")]
pub mod service;

#[cfg(feature = "sdk")]
pub mod sdk;

pub use types::catalog::{ALL_CATALOGS, CatalogEntry, CatalogId, Manifest, UnknownCatalogId};
pub use types::document::{Document, DocumentMismatch};
pub use types::java::{JavaCatalog, JavaMajor, JavaVersion};
pub use types::minecraft::{BuildSet, Distribution, MinecraftCatalog, MinecraftVersion};
pub use types::proxy::{ProxyCatalog, VelocityVersion};
pub use types::runtime::RuntimeCatalog;
pub use types::status::{CatalogStatus, StatusReport};
pub use types::version::Version;

/// Version of the wire format described by this crate.
///
/// Consumers send it as the `schema` query parameter; a server that cannot serve that
/// schema answers `409 Conflict` rather than returning a document the consumer would
/// misread. Bump on any change to a catalog document that is not purely additive.
pub const SCHEMA_VERSION: u32 = 1;
