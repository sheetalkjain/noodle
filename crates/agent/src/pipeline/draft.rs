use noodle_core::error::{NoodleError, Result};
use storage::sqlite::SqliteStorage;
use storage::qdrant::QdrantStorage;
use ai::provider::{AiProvider, ChatRequest, Message};
use std::sync::Arc;

pub struct DraftAssistant {
    sqlite: Arc<SqliteStorage>,
    qdrant: Arc<QdrantStorage>,
    ai: Arc<dyn AiProvider>,
}

impl DraftAssistant {
    pub fn new(sqlite: Arc<SqliteStorage>, qdrant: Arc<QdrantStorage>, ai: Arc<dyn AiProvider>) -> Self {
        Self { sqlite, qdrant, ai }
    }

    pub async fn generate_draft(&self, email_id: i64) -> Result<String> {
        // 1. Fetch email and facts from SQLite
        // let email = self.sqlite.get_email(email_id).await?;
        // let facts = self.sqlite.get_facts(email_id).await?;
        
        // 2. Fetch similar emails from Qdrant
        // let embedding = self.ai.generate_embedding(&email.body_text).await?;
        // let similar = self.qdrant.search_emails(embedding, None, 3).await?;

        // 3. Build grounded prompt
        let prompt = format!(
            "System: You are an AI assistant. Draft a reply to the following email. 
            Use the provided context and facts. Do not invent facts.
            Context: [Thread Content]
            Facts: [Extracted Facts]
            Similar Examples: [Vector Search Results]
            
            Email to reply to: [Body]"
        );

        let request = ChatRequest {
            messages: vec![Message { role: "user".into(), content: prompt }],
            temperature: 0.7,
            response_format: None,
        };

        let res = self.ai.chat_completion(request).await?;
        Ok(res.content)
    }
}
