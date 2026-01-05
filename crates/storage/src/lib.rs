//! Storage crate for Noodle application persistence.
//!
//! This crate provides storage implementations for emails, entities, facts,
//! and vector embeddings. It supports both relational (SQLite) and vector
//! (Qdrant) databases.
//!
//! # Modules
//! - [`sqlite`] - SQLite storage for structured data (emails, entities, facts, logs)
//! - [`qdrant`] - Qdrant vector database for semantic search embeddings
//!
//! # Data Model
//! ```text
//! ┌─────────────┐     ┌─────────────┐     ┌─────────────┐
//! │   emails    │────▶│   facts     │     │   entities  │
//! └─────────────┘     └─────────────┘     └─────────────┘
//!        │                                       │
//!        │           ┌─────────────────┐         │
//!        └──────────▶│ entity_mentions │◀────────┘
//!                    └─────────────────┘
//!                            │
//!                    ┌───────┴───────┐
//!                    ▼               ▼
//!               ┌─────────┐    ┌─────────┐
//!               │  edges  │    │  logs   │
//!               └─────────┘    └─────────┘
//! ```

pub mod qdrant;
pub mod sqlite;
