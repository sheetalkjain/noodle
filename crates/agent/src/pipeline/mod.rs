pub mod draft;

use ai::provider::{AiProvider, ChatRequest, Message};
use chrono::Utc;
use noodle_core::error::Result;
use noodle_core::types::{
    Email, EmailFact, Intent, PrimaryType, ProjectInfo, Provenance, Sentiment, Urgency, WaitingOn,
};
use std::sync::Arc;
use storage::qdrant::QdrantStorage;
use storage::sqlite::SqliteStorage;
use tracing::info;
use uuid::Uuid;

use tokio::sync::RwLock;

pub struct ExtractionPipeline {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<RwLock<Arc<dyn AiProvider>>>,
}

impl ExtractionPipeline {
    pub fn new(
        sqlite: Arc<SqliteStorage>,
        qdrant: Arc<QdrantStorage>,
        ai: Arc<RwLock<Arc<dyn AiProvider>>>,
    ) -> Self {
        Self { sqlite, qdrant, ai }
    }

    pub async fn process_email(&self, mut email: Email) -> Result<()> {
        info!("Processing email: {}", email.subject);

        // 0. Compute hash
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(&email.subject);
        hasher.update(&email.sender);
        hasher.update(&email.body_text);
        email.hash = format!("{:x}", hasher.finalize());

        // 1. Persist to SQLite first to get internal ID
        let id = self.sqlite.save_email(&email).await?;
        email.id = id;

        // 2. Extract facts using AI
        let mut facts = self.extract_facts(&email).await?;
        facts.email_id = id;

        // 3. Save facts to SQLite
        self.sqlite.save_facts(&facts).await?;

        // 4. Generate embeddings
        let ai = self.ai.read().await;
        let embedding = ai.generate_embedding(&email.body_text).await?;

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
            "Analyze the following email and extract structured project health signals.
You must assign the email to exactly one client_or_project.
Classify the primary_type, intent, urgency, and sentiment carefully based on the rules.
Extract risks, issues, blockers, and questions.

Rules:
- primary_type: 'update' (status/progress), 'request' (action required), 'decision' (announcement/approval), 'fyi' (informational).
- intent: 'inform', 'ask', 'escalate', 'commit', 'clarify', 'resolve'.
- urgency: 'low', 'medium', 'high'.
- sentiment: 'neutral', 'positive', 'concerned', 'hostile'.
- waiting_on: 'me', 'them', 'third_party', 'none'.
- severity: 'low', 'medium', 'high'.
- due_by: ISO8601 string or null.

Respond ONLY with valid JSON matching this schema:
{{
  \"primary_type\": \"update|request|decision|fyi\",
  \"intent\": \"inform|ask|escalate|commit|clarify|resolve\",
  \"urgency\": \"low|medium|high\",
  \"due_by\": \"YYYY-MM-DDTHH:MM:SSZ\" or null,
  \"sentiment\": \"neutral|positive|concerned|hostile\",
  \"client_or_project\": {{ \"name\": \"string\", \"confidence\": 0.0-1.0 }},
  \"risks\": [
    {{ \"title\": \"string\", \"details\": \"string\", \"owner\": \"string|null\", \"severity\": \"low|medium|high\", \"confidence\": 0.0-1.0 }}
  ],
  \"issues\": [
    {{ \"title\": \"string\", \"details\": \"string\", \"owner\": \"string|null\", \"severity\": \"low|medium|high\", \"confidence\": 0.0-1.0 }}
  ],
  \"blockers\": [
    {{ \"title\": \"string\", \"details\": \"string\", \"owner\": \"string|null\", \"severity\": \"low|medium|high\", \"confidence\": 0.0-1.0 }}
  ],
  \"open_questions\": [
    {{ \"question\": \"string\", \"asked_by\": \"string|null\", \"owner\": \"string|null\", \"due_by\": \"YYYY-MM-DDTHH:MM:SSZ\" or null, \"confidence\": 0.0-1.0 }}
  ],
  \"answered_questions\": [
    {{ \"question\": \"string\", \"answer_summary\": \"string\", \"confidence\": 0.0-1.0 }}
  ],
  \"needs_response\": true|false,
  \"waiting_on\": \"me|them|third_party|none\",
  \"summary\": \"string (max 80 words)\",
  \"key_points\": [\"string\"],
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
            model: None,
        };

        let ai = self.ai.read().await;
        // Retry logic could be added here
        let response = ai.chat_completion(request).await?;

        // Attempt to parse directly into EmailFact-compatible struct or generic Value then map
        // We parse to Value first to handle defaults/errors gracefully
        let fact_data: serde_json::Value =
            serde_json::from_str(&response.content).map_err(|e: serde_json::Error| {
                noodle_core::error::NoodleError::AI(format!(
                    "JSON Parse Error: {} Content: {}",
                    e, response.content
                ))
            })?;

        // Helper to parse enums defaults
        let primary_type = serde_json::from_value(fact_data["primary_type"].clone())
            .unwrap_or(noodle_core::types::PrimaryType::Fyi);
        let intent = serde_json::from_value(fact_data["intent"].clone())
            .unwrap_or(noodle_core::types::Intent::Inform);
        let urgency = serde_json::from_value(fact_data["urgency"].clone())
            .unwrap_or(noodle_core::types::Urgency::Low);
        let sentiment = serde_json::from_value(fact_data["sentiment"].clone())
            .unwrap_or(noodle_core::types::Sentiment::Neutral);
        let waiting_on = serde_json::from_value(fact_data["waiting_on"].clone())
            .unwrap_or(noodle_core::types::WaitingOn::None);

        let due_by_str = fact_data["due_by"].as_str();
        let due_by = due_by_str.and_then(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|dt| dt.with_timezone(&Utc))
        });

        Ok(EmailFact {
            email_id: email.id,
            primary_type,
            intent,
            client_or_project: serde_json::from_value(fact_data["client_or_project"].clone())
                .unwrap_or(ProjectInfo {
                    name: "Unknown".into(),
                    confidence: 0.0,
                }),
            sentiment,
            urgency,
            due_by,
            needs_response: fact_data["needs_response"].as_bool().unwrap_or(false),
            waiting_on,
            summary: fact_data["summary"].as_str().unwrap_or("").into(),
            key_points: serde_json::from_value(fact_data["key_points"].clone()).unwrap_or_default(),
            risks: serde_json::from_value(fact_data["risks"].clone()).unwrap_or_default(),
            issues: serde_json::from_value(fact_data["issues"].clone()).unwrap_or_default(),
            blockers: serde_json::from_value(fact_data["blockers"].clone()).unwrap_or_default(),
            open_questions: serde_json::from_value(fact_data["open_questions"].clone())
                .unwrap_or_default(),
            answered_questions: serde_json::from_value(fact_data["answered_questions"].clone())
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
