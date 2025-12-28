pub mod draft;

use ai::provider::{AiProvider, ChatRequest, Message};
use chrono::Utc;
use noodle_core::error::Result;
use noodle_core::types::{
    ActionItem, Email, EmailFact, EmailType, ProjectInfo, Provenance, Sentiment, Urgency,
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

    pub async fn process_email(&self, mut email: Email) -> Result<()> {
        info!("Processing email: {}", email.subject);

        // 1. Persist to SQLite first to get internal ID
        let id = self.sqlite.save_email(&email).await?;
        email.id = id;

        // 2. Extract facts using AI
        let mut facts = self.extract_facts(&email).await?;
        facts.email_id = id;

        // 3. Save facts to SQLite
        self.sqlite.save_facts(&facts).await?;

        // 4. Generate embeddings
        let embedding = self.ai.generate_embedding(&email.body_text).await?;

        // 5. Persist to Qdrant
        let payload = qdrant_client::Payload::new(); // Add metadata
        self.qdrant
            .upsert_email_vector(&email.store_id, &email.entry_id, embedding, payload)
            .await?;

        info!("Successfully processed email: {}", email.id);
        Ok(())
    }

    async fn extract_facts(&self, email: &Email) -> Result<EmailFact> {
        let prompt = format!(
            "Analyze the following email and extract key points, action items, sentiment, and urgency.
Respond ONLY with a JSON object matching this schema:
{{
  \"email_type\": \"status_update|scheduling|question|request|approval|invoice|legal|sales|support|personal|other\",
  \"project\": {{ \"name\": \"string\", \"confidence\": 0.0-1.0 }},
  \"sentiment\": \"very_negative|negative|neutral|positive|very_positive\",
  \"urgency\": \"low|medium|high|critical\",
  \"summary\": \"string\",
  \"key_points\": [\"string\"],
  \"action_items\": [\"string\"],
  \"decisions\": [\"string\"],
  \"risks\": [\"string\"],
  \"deadlines\": [\"string\"],
  \"needs_response\": true|false,
  \"suggested_labels\": [\"string\"],
  \"confidence\": 0.0-1.0
}}

Subject: {}
From: {}
Body: {}",
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

        Ok(EmailFact {
            email_id: email.id,
            email_type: serde_json::from_value(fact_data["email_type"].clone())
                .unwrap_or(EmailType::Other),
            project: serde_json::from_value(fact_data["project"].clone()).unwrap_or(ProjectInfo {
                name: "Default".into(),
                confidence: 1.0,
            }),
            sentiment: serde_json::from_value(fact_data["sentiment"].clone())
                .unwrap_or(Sentiment::Neutral),
            urgency: serde_json::from_value(fact_data["urgency"].clone())
                .unwrap_or(Urgency::Medium),
            summary: fact_data["summary"].as_str().unwrap_or("").into(),
            key_points: fact_data["key_points"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            action_items: fact_data["action_items"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| {
                            v.as_str().map(|s| ActionItem {
                                owner: None,
                                task: s.to_string(),
                                due_date: None,
                                confidence: 1.0,
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
            decisions: fact_data["decisions"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            risks: fact_data["risks"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            deadlines: fact_data["deadlines"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            needs_response: fact_data["needs_response"].as_bool().unwrap_or(false),
            suggested_labels: fact_data["suggested_labels"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default(),
            confidence: fact_data["confidence"].as_f64().unwrap_or(0.0) as f32,
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
