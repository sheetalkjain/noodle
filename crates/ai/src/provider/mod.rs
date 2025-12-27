pub mod creds;

use noodle_core::error::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[async_trait]
pub trait AiProvider: Send + Sync {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse>;
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub response_format: Option<ResponseFormat>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    #[serde(rename = "json_object")]
    Json,
    #[serde(rename = "text")]
    Text,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    pub content: String,
    pub usage: Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
}

pub struct LocalProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
}

impl LocalProvider {
    pub fn new(base_url: String, api_key: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
        }
    }
}

#[async_trait]
impl AiProvider for LocalProvider {
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut builder = self.client.post(&url);
        
        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }
        
        let response = builder
            .json(&request)
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;
            
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;
            
        // Extract content and usage from OpenAI response format
        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| noodle_core::error::NoodleError::AI("Invalid AI response format".into()))?
            .to_string();
            
        let usage = Usage {
            prompt_tokens: body["usage"]["prompt_tokens"].as_u64().unwrap_or(0) as u32,
            completion_tokens: body["usage"]["completion_tokens"].as_u64().unwrap_or(0) as u32,
        };
        
        Ok(ChatResponse { content, usage })
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/embeddings", self.base_url);
        let mut builder = self.client.post(&url);
        
        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }
        
        let response = builder
            .json(&serde_json::json!({
                "input": text,
                "model": "text-embedding-3-small"
            }))
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;
            
        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;
            
        let embedding: Vec<f32> = serde_json::from_value(body["data"][0]["embedding"].clone())
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;
            
        Ok(embedding)
    }
}
