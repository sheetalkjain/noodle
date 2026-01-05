//! Outlook email client crate for the Noodle application.
//!
//! This crate provides platform-specific implementations for reading emails from
//! Microsoft Outlook. It supports both Windows (via COM automation) and macOS
//! (via AppleScript).
//!
//! # Architecture
//! - `client` - Platform-agnostic `OutlookClient` that delegates to OS-specific implementations
//! - `com` - Windows-only COM wrapper for IDispatch automation (only compiled on Windows)
//!
//! # Platform Support
//! - **Windows**: Uses COM/IDispatch to interact with Outlook's automation interface
//! - **macOS**: Uses AppleScript via `osascript` to interact with Outlook for Mac
//!
//! # Usage
//! ```ignore
//! let client = OutlookClient::new()?;
//! let emails = client.get_emails_last_n_days(30, 6, "Inbox").await?;
//! ```

pub mod client;

#[cfg(windows)]
pub mod com;
