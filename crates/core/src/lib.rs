//! Core types and error definitions for the Noodle application.
//!
//! This crate contains shared types used across all other crates in the Noodle
//! workspace. It has no dependencies on other Noodle crates, making it the
//! foundation of the dependency graph.
//!
//! # Modules
//! - [`error`] - Error types and `Result` alias
//! - [`types`] - Domain types (Email, EmailFact, entities, enums)
//!
//! # Design Philosophy
//! - All types are serializable/deserializable with Serde
//! - Enums have sensible defaults for parsing
//! - All types implement Clone and Debug

pub mod error;
pub mod types;
