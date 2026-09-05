//! On-disk persistence for resolved catalogs.
//!
//! Persisting matters more than it looks. Without it a restart re-resolves every catalog
//! immediately, so a crash loop turns into a burst of upstream traffic — exactly the
//! behaviour that gets an instance rate limited or blocked. With it, a restarted server
//! serves the previous documents at once and only contacts upstream when they age out.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use warehouse_common::CatalogId;
use warehouse_common::types::document::Document;

/// A resolved catalog, as it sits in memory and on disk.
#[derive(Debug, Clone)]
pub(crate) struct Held {
    /// The document being served.
    pub(crate) document: Arc<Document>,
    /// Identifies this exact document, so a consumer can tell whether it changed.
    pub(crate) etag: String,
    /// When the document was built from upstream.
    pub(crate) updated_at: DateTime<Utc>,
}

impl Held {
    /// Wraps a document, deriving its etag from the encoded form.
    ///
    /// # Errors
    ///
    /// Fails if the document cannot be encoded, which would mean a bug in a schema type.
    pub(crate) fn new(document: Document) -> Result<Self, serde_json::Error> {
        let encoded = serde_json::to_vec(&document)?;
        let digest = Sha256::digest(&encoded);

        Ok(Self {
            updated_at: document.updated_at(),
            document: Arc::new(document),
            etag: format!("\"{digest:x}\""),
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct Envelope {
    schema: u32,
    updated_at: DateTime<Utc>,
    document: serde_json::Value,
}

/// Reads and writes catalogs under a data directory.
#[derive(Debug, Clone)]
pub(crate) struct Store {
    root: PathBuf,
}

impl Store {
    /// Opens a store rooted at `root`, creating the directory when it does not exist.
    ///
    /// # Errors
    ///
    /// Fails if the directory cannot be created.
    pub(crate) async fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        tokio::fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    fn path(&self, id: CatalogId) -> PathBuf {
        self.root.join(format!("{}.json", id.as_str()))
    }

    /// Loads a previously persisted catalog, if one is present and still readable.
    ///
    /// A document written by an instance serving a different schema is ignored rather
    /// than returned, so an upgrade never serves a body the new code would misread.
    pub(crate) async fn load(&self, id: CatalogId, schema: u32) -> Option<Held> {
        let raw = tokio::fs::read(self.path(id)).await.ok()?;

        let envelope: Envelope = serde_json::from_slice(&raw)
            .inspect_err(|err| {
                tracing::warn!(catalog = %id, error = %err, "ignoring unreadable stored catalog");
            })
            .ok()?;

        if envelope.schema != schema {
            tracing::info!(
                catalog = %id,
                stored = envelope.schema,
                expected = schema,
                "ignoring stored catalog written for another schema"
            );
            return None;
        }

        let document = Document::from_value(id, envelope.document)
            .inspect_err(|err| {
                tracing::warn!(catalog = %id, error = %err, "ignoring stored catalog");
            })
            .ok()?;

        Held::new(document)
            .inspect_err(|err| {
                tracing::warn!(catalog = %id, error = %err, "could not re-encode stored catalog");
            })
            .ok()
    }

    /// Persists a catalog, replacing any previous copy atomically.
    ///
    /// # Errors
    ///
    /// Fails if the document cannot be encoded or the file cannot be written.
    pub(crate) async fn save(&self, id: CatalogId, schema: u32, held: &Held) -> io::Result<()> {
        let envelope = Envelope {
            schema,
            updated_at: held.updated_at,
            document: serde_json::to_value(&*held.document)?,
        };
        let encoded = serde_json::to_vec(&envelope)?;

        let target = self.path(id);
        let temporary = target.with_extension("json.tmp");
        tokio::fs::write(&temporary, &encoded).await?;
        tokio::fs::rename(&temporary, &target).await
    }
}
