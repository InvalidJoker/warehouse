use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest as _, Sha256};
use std::io;
use std::path::PathBuf;
use std::sync::Arc;
use warehouse_common::CatalogId;
use warehouse_common::types::document::Document;

#[derive(Debug, Clone)]
pub(crate) struct Held {
    pub(crate) document: Arc<Document>,
    pub(crate) etag: String,
    pub(crate) updated_at: DateTime<Utc>,
}

impl Held {
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

#[derive(Debug, Clone)]
pub(crate) struct Store {
    root: PathBuf,
}

impl Store {
    pub(crate) async fn open(root: impl Into<PathBuf>) -> io::Result<Self> {
        let root = root.into();
        tokio::fs::create_dir_all(&root).await?;
        Ok(Self { root })
    }

    fn path(&self, id: CatalogId) -> PathBuf {
        self.root.join(format!("{}.json", id.as_str()))
    }

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
