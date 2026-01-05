//! AI provider implementations and abstractions.
//!
//! This module defines the `AiProvider` trait and provides implementations
//! for different AI backends.
//!
//! # Implementations
//! - [`OllamaProvider`] - Local Ollama server (default model: llama3)
//! - [`OpenAICompatibleProvider`] - Any OpenAI-compatible API
//!
//! # Usage
//! ```ignore
//! let provider = OllamaProvider::new("http://localhost:11434".into(), None);
//! let response = provider.chat_completion(request).await?;
//! let embedding = provider.generate_embedding("some text").await?;
//! ```

pub mod creds;

use async_trait::async_trait;
use noodle_core::error::Result;
use serde::{Deserialize, Serialize};

/// Trait for AI providers that can do chat completion and embedding generation.
///
/// Implementations must be thread-safe (`Send + Sync`) for use in async contexts.
#[async_trait]
pub trait AiProvider: Send + Sync {
    /// Generates a chat completion response from the AI model.
    ///
    /// Used for structured fact extraction from emails.
    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse>;

    /// Generates a vector embedding for the given text.
    ///
    /// Used for semantic search in the vector database.
    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>>;

    /// Lists available models from the provider.
    async fn list_models(&self) -> Result<Vec<String>>;
}

/// Request structure for chat completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatRequest {
    /// Conversation messages (system, user, assistant)
    pub messages: Vec<Message>,
    /// Sampling temperature (0.0 = deterministic, higher = more random)
    pub temperature: f32,
    /// Optional response format (JSON or text)
    pub response_format: Option<ResponseFormat>,
    /// Model name (optional, uses provider default if not specified)
    pub model: Option<String>,
}

/// A single message in a chat conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "system", "user", or "assistant"
    pub role: String,
    /// Message content
    pub content: String,
}

/// Response format specification for chat completions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ResponseFormat {
    /// Request JSON output
    #[serde(rename = "json_object")]
    Json,
    /// Request plain text output
    #[serde(rename = "text")]
    Text,
}

/// Response from a chat completion request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatResponse {
    /// The generated text content
    pub content: String,
    /// Token usage statistics
    pub usage: Usage,
}

/// Token usage statistics from an AI request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    /// Tokens in the prompt/input
    pub prompt_tokens: u32,
    /// Tokens in the completion/output
    pub completion_tokens: u32,
}

/// Available AI provider types.
pub enum ProviderType {
    /// Local Ollama server
    Ollama,
    /// OpenAI-compatible API (Lemonade, Foundry, etc.)
    OpenAICompatible,
}

pub struct OllamaProvider {
    client: reqwest::Client,
    base_url: String,
    model_name: Option<String>,
}

impl OllamaProvider {
    pub fn new(base_url: String, model_name: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            model_name,
        }
    }
}

#[async_trait]
impl AiProvider for OllamaProvider {
    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let models = body["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        Ok(models)
    }

    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/api/chat", self.base_url);

        let model = request
            .model
            .or(self.model_name.clone())
            .unwrap_or_else(|| "llama3".to_string());

        // Ollama specific request format
        let ollama_req = serde_json::json!({
            "model": model,
            "messages": request.messages,
            "stream": false,
            "format": match request.response_format {
                Some(ResponseFormat::Json) => "json",
                _ => "",
            }
        });

        let response = self
            .client
            .post(&url)
            .json(&ollama_req)
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let status = response.status();
        let body: serde_json::Value = response.json().await.map_err(|e| {
            noodle_core::error::NoodleError::AI(format!("Failed to parse Ollama response: {}", e))
        })?;

        // Check for Ollama error response
        if let Some(error) = body.get("error").and_then(|e| e.as_str()) {
            return Err(noodle_core::error::NoodleError::AI(format!(
                "Ollama error: {}",
                error
            )));
        }

        if !status.is_success() {
            return Err(noodle_core::error::NoodleError::AI(format!(
                "Ollama returned status {}: {:?}",
                status, body
            )));
        }

        let content = body["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                noodle_core::error::NoodleError::AI(format!(
                    "Invalid Ollama response format. Expected message.content, got: {}",
                    serde_json::to_string_pretty(&body).unwrap_or_else(|_| "unknown".to_string())
                ))
            })?
            .to_string();

        // approximate usage as ollama might specifically return it elsewhere
        let usage = Usage {
            prompt_tokens: body["prompt_eval_count"].as_u64().unwrap_or(0) as u32,
            completion_tokens: body["eval_count"].as_u64().unwrap_or(0) as u32,
        };

        Ok(ChatResponse { content, usage })
    }

    async fn generate_embedding(&self, text: &str) -> Result<Vec<f32>> {
        let url = format!("{}/api/embeddings", self.base_url);
        let req = serde_json::json!({
            "model": "all-minilm", // Default embedding model usually
            "prompt": text
        });

        let response = self
            .client
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let embedding: Vec<f32> = serde_json::from_value(body["embedding"].clone())
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        Ok(embedding)
    }
}

pub struct OpenAICompatibleProvider {
    client: reqwest::Client,
    base_url: String,
    api_key: Option<String>,
    model_name: Option<String>,
}

impl OpenAICompatibleProvider {
    pub fn new(base_url: String, api_key: Option<String>, model_name: Option<String>) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url,
            api_key,
            model_name,
        }
    }
}

#[async_trait]
impl AiProvider for OpenAICompatibleProvider {
    async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/models", self.base_url); // usually /v1/models but base_url might include v1
        let mut builder = self.client.get(&url);
        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }

        let response = builder
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let models = body["data"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["id"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_else(Vec::new);

        Ok(models)
    }

    async fn chat_completion(&self, request: ChatRequest) -> Result<ChatResponse> {
        let url = format!("{}/chat/completions", self.base_url);
        let mut builder = self.client.post(&url);

        if let Some(key) = &self.api_key {
            builder = builder.bearer_auth(key);
        }

        let mut req_json = serde_json::to_value(&request).unwrap();
        if let Some(obj) = req_json.as_object_mut() {
            // Inject model if missing and configured
            if !obj.contains_key("model") {
                if let Some(m) = &self.model_name {
                    obj.insert("model".to_string(), serde_json::Value::String(m.clone()));
                }
            }
        }

        let response = builder
            .json(&req_json)
            .send()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let body: serde_json::Value = response
            .json()
            .await
            .map_err(|e| noodle_core::error::NoodleError::AI(e.to_string()))?;

        let content = body["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| {
                noodle_core::error::NoodleError::AI("Invalid AI response format".into())
            })?
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
                "model": "text-embedding-3-small" // This might be problematic if not supported by other providers
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
