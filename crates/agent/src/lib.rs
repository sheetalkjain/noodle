//! Agent crate for email synchronization and AI-powered extraction.
//!
//! This crate provides the core business logic for syncing emails from Outlook
//! and extracting structured information using AI. It coordinates between the
//! Outlook client, storage layer, and AI providers.
//!
//! # Modules
//! - [`engine`] - `SyncManager` for orchestrating email sync operations
//! - [`pipeline`] - `ExtractionPipeline` for processing emails with AI
//!
//! # Architecture
//! ```text
//! ┌─────────────┐     ┌──────────────┐     ┌─────────────┐
//! │   Outlook   │────▶│  SyncManager │────▶│   Pipeline  │
//! │   Client    │     │   (engine)   │     │ (extraction)│
//! └─────────────┘     └──────────────┘     └─────────────┘
//!                            │                    │
//!                            ▼                    ▼
//!                     ┌─────────────┐     ┌─────────────┐
//!                     │   SQLite    │     │  AI Provider│
//!                     │   Storage   │     │  (Ollama)   │
//!                     └─────────────┘     └─────────────┘
//! ```

pub mod engine;
pub mod pipeline;
