use ai::provider::{AiProvider, ChatRequest, Message};
use noodle_core::error::Result;
use std::sync::Arc;
use storage::qdrant::QdrantStorage;
use storage::sqlite::SqliteStorage;

pub struct DraftAssistant {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<dyn AiProvider>,
}

impl DraftAssistant {
    pub fn new(
        sqlite: Arc<SqliteStorage>,
        qdrant: Arc<QdrantStorage>,
        ai: Arc<dyn AiProvider>,
    ) -> Self {
        Self { sqlite, qdrant, ai }
    }

    pub async fn generate_draft(&self, email_id: i64) -> Result<String> {
        use sqlx::Row;
        // 1. Fetch email from SQLite
        let email = sqlx::query_as::<_, storage::sqlite::EmailRow>(
            "SELECT id, subject, sender, received_at, body_text FROM emails WHERE id = ?",
        )
        .bind(email_id)
        .fetch_one(self.sqlite.pool())
        .await
        .map_err(|e: sqlx::Error| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        // 2. Fetch facts (optional)
        let facts = sqlx::query("SELECT summary FROM extracted_email_facts WHERE email_id = ?")
            .bind(email_id)
            .fetch_optional(self.sqlite.pool())
            .await
            .map_err(|e: sqlx::Error| noodle_core::error::NoodleError::Storage(e.to_string()))?;

        let summary = facts
            .map(|r: sqlx::sqlite::SqliteRow| r.get::<String, _>("summary"))
            .unwrap_or_default();

        // 3. Fetch similar emails from Qdrant for style/context
        let embedding = self.ai.generate_embedding(&email.body_text).await?;
        let similar = self.qdrant.search_emails(embedding, None, 3).await?;

        let mut context = String::new();
        for point in similar {
            let payload = point.payload;
            if let Some(v) = payload.get("subject") {
                if let Some(subject) = v.as_str() {
                    context.push_str(&format!("Example Subject: {}\n", subject));
                }
            }
        }

        // 4. Build grounded prompt
        let prompt = format!(
            "Analyze the following email and draft a professional reply.
            
            Original Subject: {}
            Original From: {}
            Summary of Facts: {}
            
            Style context from similar emails:
            {}
            
            Body to reply to:
            {}
            
            Draft a reply that is concise, professional, and addresses all points in the summary.",
            email.subject, email.sender, summary, context, email.body_text
        );

        let request = ChatRequest {
            messages: vec![Message {
                role: "user".into(),
                content: prompt,
            }],
            temperature: 0.7,
            response_format: None,
        };

        let res = self.ai.chat_completion(request).await?;
        Ok(res.content)
    }
}
