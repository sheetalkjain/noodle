//! Platform-specific Outlook client implementations.
//!
//! This module provides a cross-platform `OutlookClient` that abstracts over
//! OS-specific implementations for Windows and macOS.

#[cfg(windows)]
mod windows_impl;

#[cfg(target_os = "macos")]
mod macos_impl;

use noodle_core::error::Result;
use noodle_core::types::Email;

/// Cross-platform Outlook client for fetching emails.
///
/// This struct provides a unified interface for accessing Outlook emails
/// across different operating systems. It delegates to platform-specific
/// implementations at compile time using conditional compilation.
///
/// # Supported Platforms
/// - **Windows**: Uses COM automation via `windows_impl::WindowsOutlookClient`
/// - **macOS**: Uses AppleScript via `macos_impl::MacOutlookClient`
///
/// # Example
/// ```ignore
/// let client = OutlookClient::new()?;
/// let inbox_emails = client.get_emails_last_n_days(30, 6, "Inbox").await?;
/// let sent_emails = client.get_emails_last_n_days(30, 5, "Sent Items").await?;
/// ```
#[derive(Clone)]
pub struct OutlookClient {
    #[cfg(windows)]
    inner: windows_impl::WindowsOutlookClient,

    #[cfg(target_os = "macos")]
    inner: macos_impl::MacOutlookClient,
}

impl OutlookClient {
    /// Creates a new Outlook client for the current platform.
    ///
    /// On Windows, this initializes the COM subsystem and connects to Outlook.
    /// On macOS, this verifies that Outlook is available.
    ///
    /// # Returns
    /// A new `OutlookClient` instance, or an error if Outlook is not available.
    ///
    /// # Errors
    /// - On Windows: If COM initialization fails or Outlook is not installed
    /// - On macOS: If Outlook for Mac is not installed
    /// - On other platforms: Always returns an unsupported platform error
    pub fn new() -> Result<Self> {
        #[cfg(windows)]
        {
            Ok(Self {
                inner: windows_impl::WindowsOutlookClient::new()?,
            })
        }

        #[cfg(target_os = "macos")]
        {
            Ok(Self {
                inner: macos_impl::MacOutlookClient::new()?,
            })
        }

        #[cfg(not(any(windows, target_os = "macos")))]
        {
            Err(noodle_core::error::NoodleError::Outlook(
                "Outlook client is only supported on Windows and macOS".to_string(),
            ))
        }
    }

    /// Fetches emails from a specific folder within the last N days.
    ///
    /// # Arguments
    /// * `days` - Number of days to look back (e.g., 30 for last month)
    /// * `folder_id` - Outlook folder ID (6 = Inbox, 5 = Sent Items on Windows)
    /// * `folder_name` - Human-readable folder name (used on macOS)
    ///
    /// # Returns
    /// A vector of `Email` structs containing the fetched emails.
    ///
    /// # Platform Notes
    /// - On Windows, `folder_id` is used to identify the folder
    /// - On macOS, `folder_name` is used with AppleScript
    pub async fn get_emails_last_n_days(
        &self,
        days: i64,
        folder_id: i32,
        folder_name: &str,
    ) -> Result<Vec<Email>> {
        self.inner
            .get_emails_last_n_days(days, folder_id, folder_name)
            .await
    }
}
