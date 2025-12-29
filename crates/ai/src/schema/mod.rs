use crate::provider::{AiProvider, ChatRequest, Message, ResponseFormat};
use jsonschema::JSONSchema;
use noodle_core::error::{NoodleError, Result};
use serde_json::Value;
use std::sync::Arc;
use tracing::warn;

pub struct ExtractionValidator {
    schema: JSONSchema,
}

impl ExtractionValidator {
    pub fn new() -> Self {
        let schema_json = serde_json::json!({
            "type": "object",
            "properties": {
                "email_type": { "type": "string" },
                "summary": { "type": "string", "maxLength": 500 },
                "confidence": { "type": "number", "minimum": 0, "maximum": 1 },
                "needs_response": { "type": "boolean" }
            },
            "required": ["email_type", "summary", "confidence", "needs_response"]
        });

        let schema = JSONSchema::compile(&schema_json).expect("Invalid internal schema");
        Self { schema }
    }

    pub fn validate(&self, json: &Value) -> bool {
        self.schema.is_valid(json)
    }
}

pub struct ExtractionPipeline {
    ai: Arc<dyn AiProvider>,
    validator: ExtractionValidator,
}

impl ExtractionPipeline {
    pub fn new(ai: Arc<dyn AiProvider>) -> Self {
        Self {
            ai,
            validator: ExtractionValidator::new(),
        }
    }

    pub async fn extract_with_repair(&self, text: &str) -> Result<Value> {
        let mut response = self.run_extraction(text, None).await?;

        if !self.validator.validate(&response) {
            warn!("First AI response failed validation. Attempting repair pass...");
            response = self.run_repair(text, &response).await?;
        }

        if !self.validator.validate(&response) {
            return Err(NoodleError::AI(
                "Self-repair failed to produce valid JSON".into(),
            ));
        }

        Ok(response)
    }

    async fn run_extraction(
        &self,
        text: &str,
        system_prompt_override: Option<String>,
    ) -> Result<Value> {
        let system_prompt = system_prompt_override.unwrap_or_else(|| {
            "You are an expert email analyst. Output valid JSON only.".to_string()
        });

        let request = ChatRequest {
            messages: vec![
                Message {
                    role: "system".into(),
                    content: system_prompt,
                },
                Message {
                    role: "user".into(),
                    content: text.to_string(),
                },
            ],
            temperature: 0.0,
            response_format: Some(ResponseFormat::Json),
            model: None,
        };

        let res = self.ai.chat_completion(request).await?;
        serde_json::from_str(&res.content).map_err(|e| NoodleError::AI(e.to_string()))
    }

    async fn run_repair(&self, text: &str, invalid_json: &Value) -> Result<Value> {
        let repair_prompt = format!(
            "The previous JSON output was invalid according to the schema. Fix it.\n\nText: {}\n\nInvalid JSON: {}",
            text, invalid_json
        );

        self.run_extraction(
            &repair_prompt,
            Some("You are a JSON repair specialist. Output corrected JSON only.".into()),
        )
        .await
    }
}
