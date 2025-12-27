pub mod draft;

use ai::provider::{AiProvider, ChatRequest, Message};
use chrono::Utc;
use noodle_core::error::Result;
use noodle_core::types::{
    Email, EmailFact, EmailType, ProjectInfo, Provenance, Sentiment, Urgency,
};
use std::sync::Arc;
use storage::qdrant::QdrantStorage;
use storage::sqlite::SqliteStorage;
use tracing::info;
use uuid::Uuid;

pub struct ExtractionPipeline {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<dyn AiProvider>,
}

impl ExtractionPipeline {
    pub fn new(
        sqlite: Arc<SqliteStorage>,
        qdrant: Arc<QdrantStorage>,
        ai: Arc<dyn AiProvider>,
    ) -> Self {
        Self { sqlite, qdrant, ai }
    }

    pub async fn process_email(&self, email: Email) -> Result<()> {
        info!("Processing email: {}", email.subject);

        // 1. Extract facts using AI
        let _facts = self.extract_facts(&email).await?;

        // 2. Generate embeddings
        let embedding = self.ai.generate_embedding(&email.body_text).await?;

        // 3. Persist to SQLite
        // (Implementation of save_email and save_facts in storage crate)

        // 4. Persist to Qdrant
        let payload = qdrant_client::Payload::new(); // Add metadata
        self.qdrant
            .upsert_email_vector(&email.store_id, &email.entry_id, embedding, payload)
            .await?;

        info!("Successfully processed email: {}", email.id);
        Ok(())
    }

    async fn extract_facts(&self, email: &Email) -> Result<EmailFact> {
        let prompt = format!(
            "Analyze the following email and extract key points, action items, sentiment, and urgency.\n\nSubject: {}\nFrom: {}\nBody: {}",
            email.subject, email.sender, email.body_text
        );

        let request = ChatRequest {
            messages: vec![Message {
                role: "user".into(),
                content: prompt,
            }],
            temperature: 0.0,
            response_format: Some(ai::provider::ResponseFormat::Json),
        };

        let response = self.ai.chat_completion(request).await?;
        let fact_data: serde_json::Value = serde_json::from_str(&response.content)
            .map_err(|e: serde_json::Error| noodle_core::error::NoodleError::AI(e.to_string()))?;

        // Map fact_data to EmailFact struct (omitted for brevity in initial slice)
        Ok(EmailFact {
            email_id: email.id,
            email_type: EmailType::Other,
            project: ProjectInfo {
                name: "Default".into(),
                confidence: 1.0,
            },
            sentiment: Sentiment::Neutral,
            urgency: Urgency::Medium,
            summary: "Summary placeholder".into(),
            key_points: vec![],
            action_items: vec![],
            decisions: vec![],
            risks: vec![],
            deadlines: vec![],
            needs_response: false,
            suggested_labels: vec![],
            confidence: 1.0,
            provenance: Provenance {
                model: "local".into(),
                provider: "local".into(),
                prompt_id: Uuid::new_v4(),
                created_at: Utc::now(),
            },
            created_at: Utc::now(),
        })
    }
}
