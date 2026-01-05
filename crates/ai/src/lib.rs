//! AI provider crate for LLM integrations.
//!
//! This crate provides abstractions and implementations for AI providers
//! used by Noodle for fact extraction and embedding generation.
//!
//! # Modules
//! - [`provider`] - AI provider trait and implementations (Ollama, OpenAI-compatible)
//! - [`schema`] - JSON schemas for structured AI outputs
//!
//! # Supported Providers
//! - **Ollama**: Local LLM inference (default, `http://localhost:11434`)
//! - **OpenAI-compatible**: Any API following OpenAI's format (Lemonade, Foundry, etc.)
//!
//! # Features
//! - Chat completion for fact extraction
//! - Embedding generation for semantic search
//! - Model listing
//! - Hot-swappable providers via `RwLock`

pub mod provider;
pub mod schema;
