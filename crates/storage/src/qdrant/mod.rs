use core::error::Result;
use qdrant_client::qdrant::{
    vectors_config::Config, CreateCollection, DeletePoints, Distance, Filter, PointStruct,
    ScoredPoint, SearchPoints, UpsertPoints, VectorParams, VectorsConfig,
};
use qdrant_client::{Payload, Qdrant};
use sha2::{Digest, Sha256};
use std::sync::Arc;
use tracing::info;

pub const COLLECTION_EMAILS: &str = "emails";
pub const COLLECTION_ATTACHMENTS: &str = "attachments";
pub const VECTOR_NAME: &str = "body_embedding";
pub const DEFAULT_DIM: u64 = 1536;

pub struct QdrantStorage {
    client: Arc<Qdrant>,
}

impl QdrantStorage {
    pub async fn new(url: &str) -> Result<Self> {
        let client = Qdrant::from_url(url)
            .build()
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;

        let storage = Self {
            client: Arc::new(client),
        };

        storage.ensure_collections().await?;

        Ok(storage)
    }

    async fn ensure_collections(&self) -> Result<()> {
        self.ensure_collection(COLLECTION_EMAILS, 1536).await?;
        self.ensure_collection(COLLECTION_ATTACHMENTS, 1536).await?;
        Ok(())
    }

    async fn ensure_collection(&self, name: &str, dim: u64) -> Result<()> {
        if !self.client.collection_exists(name).await.unwrap_or(false) {
            info!("Creating collection: {}", name);
            self.client
                .create_collection(CreateCollection {
                    collection_name: name.into(),
                    vectors_config: Some(VectorsConfig {
                        config: Some(Config::Params(VectorParams {
                            size: dim,
                            distance: Distance::Cosine.into(),
                            ..Default::default()
                        })),
                    }),
                    ..Default::default()
                })
                .await
                .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;
        }
        Ok(())
    }

    pub async fn upsert_email_vector(
        &self,
        store_id: &str,
        entry_id: &str,
        vector: Vec<f32>,
        payload: Payload,
    ) -> Result<()> {
        let stable_id = self.calculate_stable_id(store_id, entry_id);
        let point = PointStruct::new(stable_id, vector, payload);
        self.client
            .upsert_points(UpsertPoints {
                collection_name: COLLECTION_EMAILS.into(),
                points: vec![point],
                ..Default::default()
            })
            .await
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;
        Ok(())
    }

    fn calculate_stable_id(&self, store_id: &str, entry_id: &str) -> u64 {
        let mut hasher = Sha256::new();
        hasher.update(store_id);
        hasher.update(entry_id);
        let result = hasher.finalize();
        let mut bytes = [0u8; 8];
        bytes.copy_from_slice(&result[..8]);
        u64::from_le_bytes(bytes)
    }

    pub async fn search_emails(
        &self,
        vector: Vec<f32>,
        filter: Option<Filter>,
        limit: u64,
    ) -> Result<Vec<ScoredPoint>> {
        let result = self
            .client
            .search_points(SearchPoints {
                collection_name: COLLECTION_EMAILS.into(),
                vector: vector.into(),
                filter,
                limit,
                with_payload: Some(true.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;

        Ok(result.result)
    }

    pub async fn delete_points(&self, collection: &str, filter: Filter) -> Result<()> {
        self.client
            .delete_points(DeletePoints {
                collection_name: collection.into(),
                points: Some(filter.into()),
                ..Default::default()
            })
            .await
            .map_err(|e| core::error::NoodleError::Storage(e.to_string()))?;
        Ok(())
    }
}
