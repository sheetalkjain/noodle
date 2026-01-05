//! Email extraction and processing pipeline.
//!
//! This module provides the `ExtractionPipeline` which processes emails through
//! multiple stages to extract structured information.
//!
//! # Pipeline Stages
//! 1. **Hash Computation**: Generate SHA-256 hash for deduplication
//! 2. **Persistence**: Save email to SQLite database
//! 3. **Entity Extraction**: Extract sender/recipient entities (no AI needed)
//! 4. **AI Fact Extraction**: Use LLM to extract structured facts (optional, non-fatal)
//! 5. **Embedding Generation**: Create vector embeddings for semantic search (optional)
//! 6. **Vector Storage**: Save embeddings to Qdrant (optional)
//!
//! # Error Handling
//! The pipeline is designed to be resilient:
//! - Entity extraction runs first and never fails the whole pipeline
//! - AI extraction is non-fatal - logs warnings and continues
//! - Embedding generation is non-fatal - logs warnings and continues
//!
//! # Submodules
//! - [`draft`] - Reply drafting functionality

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
use tracing::{info, warn};
use uuid::Uuid;

use tokio::sync::RwLock;

/// Pipeline for processing emails through extraction and AI analysis.
///
/// The `ExtractionPipeline` coordinates between storage and AI providers
/// to process incoming emails and extract structured information.
///
/// # Components
/// - **SQLite**: Stores emails, entities, edges, and extracted facts
/// - **Qdrant**: Stores vector embeddings for semantic search
/// - **AI Provider**: Generates facts and embeddings (Ollama or OpenAI-compatible)
///
/// # Example
/// ```ignore
/// let pipeline = ExtractionPipeline::new(sqlite, qdrant, ai);
/// pipeline.process_email(email).await?;
/// ```
pub struct ExtractionPipeline {
    /// SQLite storage for emails, entities, and facts
    sqlite: Arc<SqliteStorage>,
    /// Qdrant vector database for embeddings
    qdrant: Arc<QdrantStorage>,
    /// AI provider (behind RwLock to allow hot-swapping)
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

        // 2. Extract and save entities from email headers (always runs, no AI needed)
        // This populates the entity graph even if AI fails
        if let Err(e) = self.extract_and_save_entities(&email).await {
            warn!("Entity extraction failed for email {}: {}", email.id, e);
        }

        // 3. Extract facts using AI (non-fatal - log warning and continue if fails)
        match self.extract_facts(&email).await {
            Ok(mut facts) => {
                facts.email_id = id;
                // Save facts to SQLite
                if let Err(e) = self.sqlite.save_facts(&facts).await {
                    warn!("Failed to save facts for email {}: {}", email.id, e);
                }
            }
            Err(e) => {
                warn!(
                    "AI fact extraction failed for email '{}': {}",
                    email.subject, e
                );
                // Continue processing - email is still saved, just without AI-extracted facts
            }
        }

        // 4. Generate embeddings (non-fatal - log warning and continue if fails)
        let ai = self.ai.read().await;
        match ai.generate_embedding(&email.body_text).await {
            Ok(embedding) => {
                // 5. Persist to Qdrant
                let payload = qdrant_client::Payload::new();
                if let Err(e) = self
                    .qdrant
                    .upsert_email_vector(&email.store_id, &email.entry_id, embedding, payload)
                    .await
                {
                    warn!("Failed to save embedding for email {}: {}", email.id, e);
                }
            }
            Err(e) => {
                warn!("Embedding generation failed for email {}: {}", email.id, e);
            }
        }

        info!("Successfully processed email: {}", email.id);
        Ok(())
    }

    /// Extract entities from email headers and save them with relationships.
    async fn extract_and_save_entities(&self, email: &Email) -> Result<()> {
        info!(
            "Extracting entities for email {}: sender='{}', to='{}'",
            email.id, email.sender, email.to
        );

        // Extract sender entity
        let sender_id = self.sqlite.save_entity("person", &email.sender).await?;
        info!(
            "Created/found sender entity: id={}, name='{}'",
            sender_id, email.sender
        );

        self.sqlite
            .save_entity_mention(email.id, sender_id, "sender", 1.0)
            .await?;

        // Parse and save 'to' recipients
        let to_recipients: Vec<&str> = email
            .to
            .split(',')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();

        info!("Processing {} 'to' recipients", to_recipients.len());

        for recipient in &to_recipients {
            let recipient_id = self.sqlite.save_entity("person", recipient).await?;
            info!(
                "Created/found recipient entity: id={}, name='{}'",
                recipient_id, recipient
            );

            self.sqlite
                .save_entity_mention(email.id, recipient_id, "recipient", 1.0)
                .await?;
            // Create edge from sender to recipient
            self.sqlite
                .save_edge(sender_id, recipient_id, "sent_to", email.id)
                .await?;
            info!("Created edge: {} -> {} (sent_to)", sender_id, recipient_id);
        }

        // Parse and save 'cc' recipients if present
        if let Some(cc) = &email.cc {
            let cc_recipients: Vec<&str> = cc
                .split(',')
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .collect();

            info!("Processing {} 'cc' recipients", cc_recipients.len());

            for recipient in &cc_recipients {
                let recipient_id = self.sqlite.save_entity("person", recipient).await?;
                self.sqlite
                    .save_entity_mention(email.id, recipient_id, "cc", 1.0)
                    .await?;
                // Create edge from sender to cc recipient
                self.sqlite
                    .save_edge(sender_id, recipient_id, "cc", email.id)
                    .await?;
                info!("Created cc edge: {} -> {} (cc)", sender_id, recipient_id);
            }
        }

        info!("Entity extraction completed for email {}", email.id);
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
            // Parse array fields safely - handle null/missing values
            key_points: fact_data
                .get("key_points")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                })
                .unwrap_or_default(),
            risks: fact_data
                .get("risks")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                })
                .unwrap_or_default(),
            issues: fact_data
                .get("issues")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                })
                .unwrap_or_default(),
            blockers: fact_data
                .get("blockers")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                })
                .unwrap_or_default(),
            open_questions: fact_data
                .get("open_questions")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
                })
                .unwrap_or_default(),
            answered_questions: fact_data
                .get("answered_questions")
                .and_then(|v| {
                    if v.is_null() {
                        None
                    } else {
                        serde_json::from_value(v.clone()).ok()
                    }
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
